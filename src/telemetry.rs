use opentelemetry_sdk::propagation::{TraceContextPropagator, BaggagePropagator};
use opentelemetry_sdk::trace as sdktrace;
use opentelemetry_sdk::{runtime, Resource};
use opentelemetry::{KeyValue, global, propagation::TextMapPropagator};
use opentelemetry::propagation::composite::TextMapCompositePropagator;
use tracing_subscriber::{layer::SubscriberExt, EnvFilter, Registry};
use tracing_opentelemetry::OpenTelemetryLayer;
use opentelemetry_otlp::WithExportConfig;

pub fn init_telemetry() -> Result<sdktrace::Tracer, Box<dyn std::error::Error>> {
    // Set up multiple propagators for better compatibility
    // This includes W3C Trace Context (standard) and Baggage
    let composite_propagator = TextMapCompositePropagator::new(vec![
        Box::new(TraceContextPropagator::new()) as Box<dyn TextMapPropagator + Send + Sync>,
        Box::new(BaggagePropagator::new()) as Box<dyn TextMapPropagator + Send + Sync>,
    ]);
    global::set_text_map_propagator(composite_propagator);

    let service_name = std::env::var("SERVICE_NAME").unwrap_or_else(|_| "forum-api".to_string());
    println!("Initializing telemetry for service: {}", service_name);

    let exporter = opentelemetry_otlp::new_exporter()
        .tonic()
        .with_endpoint("http://jaeger:4317");

    // Use high sampling rate for testing - sample all traces from load testing
    let sampler = sdktrace::Sampler::TraceIdRatioBased(1.0);
    
    let trace_config = sdktrace::Config::default()
        .with_sampler(sampler)
        .with_resource(Resource::new(vec![
            KeyValue::new("service.name", service_name.clone()),
            KeyValue::new("service.version", env!("CARGO_PKG_VERSION")),
            KeyValue::new("deployment.environment", "development"),
        ]));

    let tracer = opentelemetry_otlp::new_pipeline()
        .tracing()
        .with_exporter(exporter)
        .with_trace_config(trace_config)
        .install_batch(runtime::Tokio)?;

    println!("OpenTelemetry tracer initialized successfully");

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

    println!("Tracing subscriber configured with OpenTelemetry layer");
    Ok(tracer)
}

pub fn shutdown_telemetry() {
    global::shutdown_tracer_provider();
}
