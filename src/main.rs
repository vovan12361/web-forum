
use actix_web::{web, App, HttpServer, middleware::Logger};
use actix_web::middleware::Compress;
use actix_web::get;
use actix_files::NamedFile;
use scylla::{SessionBuilder, transport::session::PoolSize};
use std::num::NonZeroUsize;
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
    NamedFile::open("./static/docs.html")
}

#[actix_web::main]
async fn main() -> io::Result<()> {
    // Initialize telemetry
    let _tracer = telemetry::init_telemetry().expect("Failed to initialize telemetry");

    // Enable logging
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));

    // Connect to ScyllaDB cluster with optimizations
    let session = Arc::new(
        SessionBuilder::new()
            .known_node("scylladb:9042") // Using docker-compose service name
            .connection_timeout(std::time::Duration::from_secs(5))
            .pool_size(PoolSize::PerHost(NonZeroUsize::new(8).unwrap()))  // 8 connections per host
            .build()
            .await
            .expect("Failed to connect to ScyllaDB")
    );

    // Initialize database
    db::init_db(&session).await.expect("Failed to initialize database");
    
    // Initialize prepared statements for better performance
    routes::init_prepared_statements(&session).await.expect("Failed to initialize prepared statements");

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
    .workers(4)  // Limit number of workers for stability
    .max_connections(1024)  // Limit max connections per worker  
    .client_request_timeout(std::time::Duration::from_secs(10))  // Request timeout
    .client_disconnect_timeout(std::time::Duration::from_secs(5))  // Disconnect timeout
    .bind("0.0.0.0:8080")?
    .run();
    
    // Run server without capturing handle to reduce overhead
    server.await
}
