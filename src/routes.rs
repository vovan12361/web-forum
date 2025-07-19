use crate::models::{Board};
use actix_web::{get, post, web, HttpResponse, Responder};
use scylla::Session;
use serde::Deserialize;
use uuid::Uuid;
use chrono::Utc;

#[derive(Deserialize)]
pub struct CreateBoard {
    pub name: String,
    pub description: String,
}

#[post("/boards")]
pub async fn create_board(
    session: web::Data<Session>,
    new_board: web::Json<CreateBoard>,
) -> impl Responder {
    let board = Board {
        id: Uuid::new_v4(),
        name: new_board.name.clone(),
        description: new_board.description.clone(),
        created_at: Utc::now(),
    };

    let result = session
        .query(
            "INSERT INTO boards (id, name, description, created_at) VALUES (?, ?, ?, ?)",
            (board.id, &board.name, &board.description, board.created_at),
        )
        .await;

    match result {
        Ok(_) => HttpResponse::Created().json(board),
        Err(e) => HttpResponse::InternalServerError().body(e.to_string()),
    }
}

#[get("/boards")]
pub async fn get_boards(session: web::Data<Session>) -> impl Responder {
    let result = session.query("SELECT id, name, description, created_at FROM boards", &[]).await;

    match result {
        Ok(rows) => {
            let boards: Vec<Board> = rows.rows_typed().unwrap().into_iter().map(|r| r.unwrap()).collect();
            HttpResponse::Ok().json(boards)
        }
        Err(e) => HttpResponse::InternalServerError().body(e.to_string()),
    }
}
