use actix_web::body::MessageBody;
use actix_web::dev::{ServiceRequest, ServiceResponse};
use actix_web::Error;
use std::future::{ready, Ready};
use std::rc::Rc;
use std::task::{Context, Poll};
use actix_web::dev::{Service, Transform};
use tracing::{info_span, Span};
use futures_util::future::LocalBoxFuture;
use uuid::Uuid;
use std::time::Instant;

// Middleware factory for tracing requests
pub struct TracingLogger;

impl<S, B> Transform<S, ServiceRequest> for TracingLogger
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
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
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
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
        let trace_id = Uuid::new_v4().to_string();

        // Create span for this request
        let path = req.path().to_owned();
        let method = req.method().to_string();
        let span = info_span!(
            "http_request",
            trace_id = %trace_id,
            method = %method,
            path = %path,
        );

        let service = Rc::clone(&self.service);

        Box::pin(async move {
            // Set request headers for tracing context
            let mut req = req;
            req.headers_mut().insert(
                "X-Trace-ID",
                trace_id.parse().expect("trace_id should be valid header value"),
            );

            // Enter the span for the duration of this request
            let _enter = span.enter();

            // Log request info
            tracing::info!(
                method = %method,
                path = %path,
                "Request started"
            );

            // Process the request and get the response
            let res = service.call(req).await?;

            // Log response info
            let status = res.status().as_u16();
            let duration = start_time.elapsed().as_millis() as u64;

            tracing::info!(
                method = %method,
                path = %path,
                status = %status,
                duration_ms = %duration,
                "Request completed"
            );

            // Add response headers
            let mut res = res;
            {
                let headers = res.headers_mut();
                headers.insert("X-Trace-ID", trace_id.parse().expect("trace_id should be valid header value"));
                headers.insert("X-Response-Time-Ms", duration.to_string().parse().expect("duration should be valid header value"));
            }

            Ok(res)
        })
    }
} 