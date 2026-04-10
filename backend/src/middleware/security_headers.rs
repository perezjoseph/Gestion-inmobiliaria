use actix_web::dev::{Service, ServiceRequest, ServiceResponse, Transform};
use std::future::{Ready, ready};
use std::pin::Pin;
use std::task::{Context, Poll};

/// Middleware que agrega cabeceras de seguridad a todas las respuestas.
pub struct SecurityHeaders;

impl<S, B> Transform<S, ServiceRequest> for SecurityHeaders
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = actix_web::Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = actix_web::Error;
    type Transform = SecurityHeadersMiddleware<S>;
    type InitError = ();
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(SecurityHeadersMiddleware { service }))
    }
}

pub struct SecurityHeadersMiddleware<S> {
    service: S,
}

impl<S, B> Service<ServiceRequest> for SecurityHeadersMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = actix_web::Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = actix_web::Error;
    type Future = Pin<Box<dyn std::future::Future<Output = Result<Self::Response, Self::Error>>>>;

    fn poll_ready(&self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.service.poll_ready(cx)
    }

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let fut = self.service.call(req);

        Box::pin(async move {
            use actix_web::http::header::{HeaderName, HeaderValue};

            let mut res = fut.await?;
            let headers = res.headers_mut();
            headers.insert(
                actix_web::http::header::X_CONTENT_TYPE_OPTIONS,
                HeaderValue::from_static("nosniff"),
            );
            headers.insert(
                actix_web::http::header::X_FRAME_OPTIONS,
                HeaderValue::from_static("DENY"),
            );
            headers.insert(
                HeaderName::from_static("x-xss-protection"),
                HeaderValue::from_static("0"),
            );
            headers.insert(
                HeaderName::from_static("content-security-policy"),
                HeaderValue::from_static("default-src 'none'; frame-ancestors 'none'"),
            );
            headers.insert(
                HeaderName::from_static("cache-control"),
                HeaderValue::from_static("no-store"),
            );
            headers.insert(
                HeaderName::from_static("referrer-policy"),
                HeaderValue::from_static("strict-origin-when-cross-origin"),
            );
            headers.insert(
                HeaderName::from_static("permissions-policy"),
                HeaderValue::from_static("camera=(), microphone=(), geolocation=()"),
            );
            headers.insert(
                HeaderName::from_static("strict-transport-security"),
                HeaderValue::from_static("max-age=31536000; includeSubDomains"),
            );
            Ok(res)
        })
    }
}
