use opentelemetry_sdk::propagation::TraceContextPropagator;
use opentelemetry_sdk::trace as sdktrace;
use opentelemetry_sdk::{runtime, Resource};
use opentelemetry::{KeyValue, global};
use tracing_subscriber::{layer::SubscriberExt, EnvFilter, Registry};
use tracing_opentelemetry::OpenTelemetryLayer;
use opentelemetry_otlp::WithExportConfig;

pub fn init_telemetry() -> Result<sdktrace::Tracer, Box<dyn std::error::Error>> {
    global::set_text_map_propagator(TraceContextPropagator::new());

    let service_name = std::env::var("SERVICE_NAME").unwrap_or_else(|_| "forum-api".to_string());

    let exporter = opentelemetry_otlp::new_exporter()
        .tonic()
        .with_endpoint("http://jaeger:4317");

    // Use sampling to reduce overhead under high load
    let sampler = sdktrace::Sampler::TraceIdRatioBased(0.1); // Sample 10% of traces
    
    let trace_config = sdktrace::Config::default()
        .with_sampler(sampler)
        .with_resource(Resource::new(vec![
            KeyValue::new("service.name", service_name),
            KeyValue::new("service.version", env!("CARGO_PKG_VERSION")),
            KeyValue::new("deployment.environment", "production"),
        ]));

    // Configure batch export with smaller batches for high load
    let batch_config = sdktrace::BatchConfig::default()
        .with_max_export_batch_size(256)      // Smaller batches
        .with_max_queue_size(1024)            // Limit queue size
        .with_scheduled_delay(std::time::Duration::from_millis(100))  // Export more frequently
        .with_max_export_timeout(std::time::Duration::from_secs(2));  // Shorter timeout

    let tracer = opentelemetry_otlp::new_pipeline()
        .tracing()
        .with_exporter(exporter)
        .with_trace_config(trace_config)
        .with_batch_config(batch_config)
        .install_batch(runtime::Tokio)?;

    // Create OpenTelemetry tracing layer
    let opentelemetry_layer = OpenTelemetryLayer::new(tracer.clone());

    // Configure logging and tracing
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    // Create subscriber with the layers
    let subscriber = Registry::default()
        .with(env_filter)
        .with(opentelemetry_layer);

    // Set subscriber as global default
    tracing::subscriber::set_global_default(subscriber)?;

    Ok(tracer)
}

pub fn shutdown_telemetry() {
    global::shutdown_tracer_provider();
}
