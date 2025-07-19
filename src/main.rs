
use actix_web::{web, App, HttpServer};
use scylla::SessionBuilder;
use std::sync::Arc;

mod db;
mod models;
mod routes;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Подключаемся к кластеру ScyllaDB
    let session = Arc::new(
        SessionBuilder::new()
            .known_node("127.0.0.1:9042")
            .build()
            .await
            .expect("Failed to connect to ScyllaDB")
    );

    // Инициализируем базу данных
    db::init_db(&session).await.expect("Failed to initialize database");

    // Запускаем веб-сервер
    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(session.clone()))
            .service(routes::create_board)
            .service(routes::get_boards)
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}
