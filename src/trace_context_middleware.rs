use actix_web::body::MessageBody;
use actix_web::dev::{ServiceRequest, ServiceResponse};
use actix_web::Error;
use std::future::{ready, Ready};
use std::rc::Rc;
use std::task::{Context, Poll};
use actix_web::dev::{Service, Transform};
use futures_util::future::LocalBoxFuture;
use opentelemetry::global;
use opentelemetry::propagation::Extractor;
use tracing_opentelemetry::OpenTelemetrySpanExt;
use actix_web::http::header::HeaderMap;

/// Custom carrier for extracting OpenTelemetry context from Actix-Web HeaderMap
struct HeaderMapCarrier<'a> {
    headers: &'a HeaderMap,
}

impl<'a> HeaderMapCarrier<'a> {
    fn new(headers: &'a HeaderMap) -> Self {
        Self { headers }
    }
}

impl<'a> Extractor for HeaderMapCarrier<'a> {
    /// Get a value from the headers by key name
    fn get(&self, key: &str) -> Option<&str> {
        self.headers.get(key)?.to_str().ok()
    }

    /// Get all header keys
    fn keys(&self) -> Vec<&str> {
        self.headers.keys().map(|name| name.as_str()).collect()
    }
}

/// Middleware for extracting OpenTelemetry trace context from HTTP headers
/// and setting it as the parent context for downstream spans created by #[instrument]
pub struct TraceContextExtractor;

impl<S, B> Transform<S, ServiceRequest> for TraceContextExtractor
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: MessageBody + 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type InitError = ();
    type Transform = TraceContextExtractorMiddleware<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(TraceContextExtractorMiddleware {
            service: Rc::new(service),
        }))
    }
}

pub struct TraceContextExtractorMiddleware<S> {
    service: Rc<S>,
}

impl<S, B> Service<ServiceRequest> for TraceContextExtractorMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: MessageBody + 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&self, ctx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.service.poll_ready(ctx)
    }

    fn call(&self, req: ServiceRequest) -> Self::Future {
        // Extract OpenTelemetry context from incoming headers
        let parent_cx = global::get_text_map_propagator(|propagator| {
            // Create a carrier that implements Extractor trait
            let carrier = HeaderMapCarrier::new(req.headers());
            propagator.extract(&carrier)
        });

        // Debug: log trace headers
        if let Some(traceparent) = req.headers().get("traceparent") {
            tracing::info!("Received traceparent header: {:?}", traceparent);
        }
        if let Some(tracestate) = req.headers().get("tracestate") {
            tracing::info!("Received tracestate header: {:?}", tracestate);
        }

        let service = Rc::clone(&self.service);

        Box::pin(async move {
            // Attach the extracted context to the current tracing span
            // This ensures that #[instrument] spans will inherit the correct parent context
            tracing::Span::current().set_parent(parent_cx);

            // Process the request
            service.call(req).await
        })
    }
}
