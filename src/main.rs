use actix_web::{web, App, HttpServer, middleware::Logger};
use actix_web::middleware::Compress;
use actix_web::get;
use actix_files::NamedFile;
use scylla::{SessionBuilder, transport::session::PoolSize};
use std::num::NonZeroUsize;
use std::sync::Arc;
use std::io;
use std::collections::HashMap;
use utoipa_swagger_ui::SwaggerUi;
use utoipa::OpenApi;
use actix_web_prom::{PrometheusMetricsBuilder};
use prometheus::{opts, IntCounterVec, Histogram, Counter, Gauge};

mod api_docs;
mod db;
mod models;
mod routes;
mod telemetry;
mod tracing_middleware;

#[get("/html-docs")]
async fn html_docs() -> io::Result<NamedFile> {
    NamedFile::open("/app/static/docs.html")
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

    // Setup Prometheus metrics with custom labels and process metrics
    let mut labels = HashMap::new();
    labels.insert("service".to_string(), "forum-api".to_string());
    labels.insert("version".to_string(), env!("CARGO_PKG_VERSION").to_string());
    
    let prometheus = PrometheusMetricsBuilder::new("forum_api")
        .endpoint("/metrics")
        .const_labels(labels)
        .build()
        .unwrap();

    // Create custom metrics for specific business logic
    let db_operations_counter = IntCounterVec::new(
        opts!("db_operations_total", "Total database operations").namespace("forum_api"),
        &["operation", "table", "status"]
    ).unwrap();
    
    let cache_operations_counter = IntCounterVec::new(
        opts!("cache_operations_total", "Cache operations by type and result").namespace("forum_api"),
        &["cache_type", "result"] // result: hit, miss, expired
    ).unwrap();
    
    let cpu_intensive_operations_counter = Counter::with_opts(
        opts!("cpu_intensive_operations_total", "Total CPU intensive operations").namespace("forum_api")
    ).unwrap();
    
    let memory_usage_gauge = Gauge::with_opts(
        opts!("process_memory_usage_bytes", "Current memory usage").namespace("forum_api")
    ).unwrap();
    
    let slow_endpoint_duration = Histogram::with_opts(
        prometheus::HistogramOpts::new(
            "slow_endpoint_duration_seconds",
            "Duration of slow endpoint operations"
        )
        .namespace("forum_api")
        .buckets(vec![0.1, 0.5, 1.0, 2.0, 5.0, 10.0, 20.0, 30.0])
    ).unwrap();

    // Register custom metrics with actix-web-prom registry
    prometheus.registry.register(Box::new(db_operations_counter.clone())).unwrap();
    prometheus.registry.register(Box::new(cache_operations_counter.clone())).unwrap();
    prometheus.registry.register(Box::new(cpu_intensive_operations_counter.clone())).unwrap();
    prometheus.registry.register(Box::new(memory_usage_gauge.clone())).unwrap();
    prometheus.registry.register(Box::new(slow_endpoint_duration.clone())).unwrap();

    println!("Starting server at http://0.0.0.0:8080");
    println!("API documentation available at http://0.0.0.0:8080/docs");
    println!("Prometheus metrics available at http://0.0.0.0:8080/metrics");
    println!("HTML documentation available at http://0.0.0.0:8080/html-docs");
    println!("actix-web-prom automatically tracks HTTP requests, duration, and status codes");

    // Generate OpenAPI documentation
    let openapi = api_docs::ApiDoc::openapi();

    // Start web server
    let server = HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(session.clone()))
            .app_data(web::Data::new(routes::DbCounter(db_operations_counter.clone())))
            .app_data(web::Data::new(routes::CacheCounter(cache_operations_counter.clone())))
            .app_data(web::Data::new(cpu_intensive_operations_counter.clone()))
            .app_data(web::Data::new(memory_usage_gauge.clone()))
            .app_data(web::Data::new(slow_endpoint_duration.clone()))
            .wrap(prometheus.clone()) // Add actix-web-prom middleware - must be first!
            .wrap(tracing_middleware::TracingLogger) // Add distributed tracing middleware
            .wrap(Logger::default())
            .wrap(Compress::default())
            // Serve Swagger UI at /docs
            .service(SwaggerUi::new("/docs/{_:.*}").url("/api-docs/openapi.json", openapi.clone()))
            // Serve HTML docs
            .service(html_docs)
            // Health endpoint (metrics endpoint is auto-registered by actix-web-prom at /metrics)
            .service(routes::health_check)
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
            // Artificial slow endpoint for testing alerts and profiling
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
