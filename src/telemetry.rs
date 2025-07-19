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

    let trace_config = sdktrace::Config::default()
        .with_sampler(sdktrace::Sampler::AlwaysOn)
        .with_resource(Resource::new(vec![
            KeyValue::new("service.name", service_name),
            KeyValue::new("service.version", env!("CARGO_PKG_VERSION")),
            KeyValue::new("deployment.environment", "production"),
        ]));

    let tracer = opentelemetry_otlp::new_pipeline()
        .tracing()
        .with_exporter(exporter)
        .with_trace_config(trace_config)
        .with_batch_config(sdktrace::BatchConfig::default())
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
