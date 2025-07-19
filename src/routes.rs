use actix_web::{get, post, web, HttpResponse, Responder};
use scylla::Session;
use crate::models::Board;
use chrono::TimeZone;

// Удалите эту строку, она здесь не нужна:
// #[derive(Deserialize)]

#[post("/boards")]
pub async fn create_board(
    session: web::Data<Session>,
    board_data: web::Json<Board>,
) -> impl Responder {
    let board = board_data.into_inner();
    
    let prepared = session.prepare("INSERT INTO boards (id, name, description, created_at) VALUES (?, ?, ?, ?)").await;
    
    if let Err(e) = prepared {
        return HttpResponse::InternalServerError().body(format!("Error preparing query: {}", e));
    }
    
    // Преобразуем DateTime в формат, который поддерживается Scylla (unix timestamp в миллисекундах)
    let created_at_timestamp = board.created_at.timestamp_millis();
    
    let result = session
        .execute(
            &prepared.unwrap(),
            (board.id, &board.name, &board.description, created_at_timestamp),
        )
        .await;

    match result {
        Ok(_) => HttpResponse::Created().json(board),
        Err(e) => HttpResponse::InternalServerError().body(format!("Error creating board: {}", e)),
    }
}

// В обработчике получения списка досок
#[get("/boards")]
pub async fn get_boards(session: web::Data<Session>) -> impl Responder {
    let prepared = match session.prepare("SELECT id, name, description, created_at FROM boards").await {
        Ok(p) => p,
        Err(e) => return HttpResponse::InternalServerError().body(format!("Error preparing query: {}", e))
    };
    
    let result = session.execute(&prepared, &[]).await;
    match result {
        Ok(rows) => {
            let boards: Vec<Board> = rows
                .rows
                .unwrap_or_default()
                .into_iter()
                .filter_map(|row| {
                    let id = row.columns[0].as_ref()?.as_uuid()?;
                    let name = row.columns[1].as_ref()?.as_text()?.to_string();
                    let description = row.columns[2].as_ref()?.as_text()?.to_string();
                    
                    // Используем timestamp_millis_opt вместо устаревшего timestamp_millis
                    let timestamp_millis = row.columns[3].as_ref()?.as_bigint()?;
                    let created_at = chrono::Utc.timestamp_millis_opt(timestamp_millis).single()?;

                    Some(Board {
                        id,
                        name,
                        description,
                        created_at,
                    })
                })
                .collect();

            HttpResponse::Ok().json(boards)
        }
        Err(e) => HttpResponse::InternalServerError().body(format!("Error fetching boards: {}", e)),
    }
}
