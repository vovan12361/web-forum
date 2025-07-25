use actix_web::body::MessageBody;
use actix_web::dev::{ServiceRequest, ServiceResponse};
use actix_web::Error;
use actix_web::http::header::{HeaderName, HeaderValue, HeaderMap};
use std::future::{ready, Ready};
use std::rc::Rc;
use std::task::{Context, Poll};
use actix_web::dev::{Service, Transform};
use futures_util::future::LocalBoxFuture;
use uuid::Uuid;
use std::time::Instant;
use opentelemetry::global;
use opentelemetry::trace::{TraceContextExt, Status, Tracer, Span};
use opentelemetry::propagation::Extractor;
use opentelemetry::{KeyValue};

// Custom header extractor for OpenTelemetry context propagation
struct HeaderExtractor<'a> {
    headers: &'a HeaderMap,
}

impl<'a> HeaderExtractor<'a> {
    fn new(headers: &'a HeaderMap) -> Self {
        Self { headers }
    }
}

impl<'a> Extractor for HeaderExtractor<'a> {
    fn get(&self, key: &str) -> Option<&str> {
        self.headers
            .get(key)
            .and_then(|value| value.to_str().ok())
    }

    fn keys(&self) -> Vec<&str> {
        self.headers
            .keys()
            .map(|name| name.as_str())
            .collect()
    }
}

// Middleware factory for tracing requests
pub struct TracingLogger;

impl<S, B> Transform<S, ServiceRequest> for TracingLogger
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: MessageBody + 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type InitError = ();
    type Transform = TracingLoggerMiddleware<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(TracingLoggerMiddleware {
            service: Rc::new(service),
        }))
    }
}

pub struct TracingLoggerMiddleware<S> {
    service: Rc<S>,
}

impl<S, B> Service<ServiceRequest> for TracingLoggerMiddleware<S>
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
        let start_time = Instant::now();
        
        // Debug: log incoming headers
        println!("Incoming headers:");
        for (name, value) in req.headers().iter() {
            if name.as_str().to_lowercase().contains("trace") || 
               name.as_str().to_lowercase().contains("baggage") ||
               name.as_str().to_lowercase().contains("x-") {
                println!("  {}: {:?}", name, value);
            }
        }
        
        // Extract OpenTelemetry context from incoming headers
        let parent_cx = global::get_text_map_propagator(|propagator| {
            let header_map = req.headers();
            let carrier = HeaderExtractor::new(header_map);
            propagator.extract(&carrier)
        });

        // Check if parent context was extracted successfully
        let parent_span = parent_cx.span();
        let parent_span_context = parent_span.span_context();
        let has_parent = parent_span_context.is_valid();
        println!("Parent context extracted: {}", has_parent);
        if has_parent {
            println!("Parent trace ID: {}", parent_span_context.trace_id());
            println!("Parent span ID: {}", parent_span_context.span_id());
        }

        // Check for load test indicators
        let is_load_test = req.headers().get("x-load-test").is_some();
        let user_agent = req.headers()
            .get("user-agent")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("unknown");

        // Create OpenTelemetry span for this request with parent context
        let path = req.path().to_owned();
        let method = req.method().to_string();
        
        let tracer = global::tracer("forum-api");
        let mut span_builder = tracer
            .span_builder(format!("{} {}", method, path))
            .with_kind(opentelemetry::trace::SpanKind::Server);

        // Set span attributes
        span_builder = span_builder
            .with_attributes(vec![
                KeyValue::new("http.method", method.clone()),
                KeyValue::new("http.route", path.clone()),
                KeyValue::new("http.scheme", "http"),
                KeyValue::new("user_agent", user_agent.to_string()),
                KeyValue::new("load_test", is_load_test),
                KeyValue::new("has_parent", has_parent),
            ]);

        // Start span with parent context
        let span = tracer.build_with_context(span_builder, &parent_cx);
        let span_context = span.span_context().clone();
        let trace_id = span_context.trace_id().to_string();

        println!("Created span with trace ID: {}", trace_id);

        let service = Rc::clone(&self.service);

        Box::pin(async move {
            // Create a new context with our span as the active span
            let cx = parent_cx.with_span(span);
            
            // Log request info
            println!(
                "Request started: {} {} (trace_id: {}, has_parent: {})", 
                method, path, trace_id, has_parent
            );

            // Process the request
            let res = service.call(req).await?;

            // Get response info
            let status = res.status().as_u16();
            let duration = start_time.elapsed().as_millis() as u64;

            // Update span with response information
            let current_span = cx.span();
            current_span.set_attribute(KeyValue::new("http.status_code", status as i64));
            current_span.set_attribute(KeyValue::new("duration_ms", duration as i64));
            
            // Set span status based on HTTP status code
            if status >= 400 {
                current_span.set_status(Status::Error {
                    description: format!("HTTP {}", status).into(),
                });
            } else {
                current_span.set_status(Status::Ok);
            }

            // End the span
            current_span.end();

            println!(
                "Request completed: {} {} - {} ({}ms, trace_id: {})",
                method, path, status, duration, trace_id
            );

            // Generate a request ID for tracing
            let request_id = Uuid::new_v4().to_string();

            // Add response headers
            let mut res = res;
            {
                let headers = res.headers_mut();
                headers.insert(
                    HeaderName::from_static("x-request-id"),
                    HeaderValue::from_str(&request_id).expect("request_id should be valid header value")
                );
                headers.insert(
                    HeaderName::from_static("x-response-time-ms"),
                    HeaderValue::from_str(&duration.to_string()).expect("duration should be valid header value")
                );
                headers.insert(
                    HeaderName::from_static("x-trace-id"),
                    HeaderValue::from_str(&trace_id).expect("trace_id should be valid header value")
                );
            }

            Ok(res)
        })
    }
} 