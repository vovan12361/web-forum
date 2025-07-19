use opentelemetry::sdk::propagation::TraceContextPropagator;
use opentelemetry::sdk::trace::{self, Sampler};
use opentelemetry::sdk::Resource;
use opentelemetry::KeyValue;
use opentelemetry_jaeger::new_agent_pipeline;
use tracing_subscriber::{layer::SubscriberExt, EnvFilter, Registry};
use tracing_opentelemetry::OpenTelemetryLayer;

pub fn init_telemetry() -> Result<(), Box<dyn std::error::Error>> {
    // Configure OpenTelemetry tracer
    opentelemetry::global::set_text_map_propagator(TraceContextPropagator::new());

    let service_name = std::env::var("SERVICE_NAME").unwrap_or_else(|_| "forum-api".to_string());

    // Configure Jaeger exporter
    let tracer = new_agent_pipeline()
        .with_service_name(service_name.clone())
        .with_trace_config(
            trace::config()
                .with_sampler(Sampler::AlwaysOn)
                .with_resource(Resource::new(vec![
                    KeyValue::new("service.name", service_name),
                    KeyValue::new("service.version", env!("CARGO_PKG_VERSION")),
                    KeyValue::new("deployment.environment", "production"),
                ])),
        )
        .install_batch(opentelemetry::runtime::Tokio)?;

    // Create OpenTelemetry tracing layer
    let opentelemetry_layer = OpenTelemetryLayer::new(tracer);

    // Configure logging and tracing
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    // Create subscriber with the layers
    let subscriber = Registry::default()
        .with(env_filter)
        .with(opentelemetry_layer);

    // Set subscriber as global default
    tracing::subscriber::set_global_default(subscriber)?;

    Ok(())
}

pub fn shutdown_telemetry() {
    // Shutdown trace pipeline gracefully
    opentelemetry::global::shutdown_tracer_provider();
} 