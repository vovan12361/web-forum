
use actix_web::{web, App, HttpServer, middleware::Logger};
use actix_web::middleware::Compress;
use actix_web::get;
use actix_files::NamedFile;
use scylla::SessionBuilder;
use std::sync::Arc;
use std::io;
use utoipa_swagger_ui::SwaggerUi;
use utoipa::OpenApi;

mod api_docs;
mod db;
mod models;
mod routes;
mod telemetry;
mod tracing_middleware;

#[get("/html-docs")]
async fn html_docs() -> io::Result<NamedFile> {
    NamedFile::open("static/docs.html")
}

#[actix_web::main]
async fn main() -> io::Result<()> {
    // Initialize telemetry
    let _tracer = telemetry::init_telemetry().expect("Failed to initialize telemetry");

    // Enable logging
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));

    // Connect to ScyllaDB cluster
    let session = Arc::new(
        SessionBuilder::new()
            .known_node("scylladb:9042") // Using docker-compose service name
            .build()
            .await
            .expect("Failed to connect to ScyllaDB")
    );

    // Initialize database
    db::init_db(&session).await.expect("Failed to initialize database");

    println!("Starting server at http://0.0.0.0:8080");
    println!("API documentation available at http://0.0.0.0:8080/docs");
    println!("HTML documentation available at http://0.0.0.0:8080/html-docs");

    // Generate OpenAPI documentation
    let openapi = api_docs::ApiDoc::openapi();

    // Start web server
    let server = HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(session.clone()))
            .wrap(Logger::default())
            .wrap(tracing_middleware::TracingLogger)
            .wrap(Compress::default())
            // Serve Swagger UI at /docs
            .service(SwaggerUi::new("/docs/{_:.*}").url("/api-docs/openapi.json", openapi.clone()))
            // Serve HTML docs
            .service(html_docs)
            // Health and metrics endpoints
            .service(routes::health_check)
            .service(routes::metrics)
            // Board related endpoints
            .service(routes::create_board)
            .service(routes::get_boards)
            .service(routes::get_board)
            // Post related endpoints
            .service(routes::create_post)
            .service(routes::get_posts_by_board)
            .service(routes::get_post)
            // Comment related endpoints
            .service(routes::create_comment)
            .service(routes::get_comments_by_post)
            // Artificial slow endpoint for testing alerts
            .service(routes::slow_endpoint)
    })
    .bind("0.0.0.0:8080")?
    .run();
    
    // Run server and capture handle
    let _server_handle = server.handle();
    let server_future = server.await;
    
    // Ensure telemetry is properly shut down
    telemetry::shutdown_telemetry();
    
    server_future
}
