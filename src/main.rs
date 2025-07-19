
use actix_web::{get, App, HttpResponse, HttpServer, Responder};
mod db;
mod models;
mod routes;

#[get("/")]
async fn hello() -> impl Responder {
    HttpResponse::Ok().body("Hello world!")
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let session = db::connect().await.expect("db connection failed");
    HttpServer::new(move || {
        App::new()
            .app_data(actix_web::web::Data::new(session.clone()))
            .service(hello)
            .service(routes::create_board)
            .service(routes::get_boards)
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}
