use opentelemetry::trace::TracerProvider;
use opentelemetry_otlp::{SpanExporter, WithExportConfig};
use opentelemetry_sdk::Resource;
use opentelemetry_sdk::trace::SdkTracerProvider;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{EnvFilter, Registry};

pub fn init_telemetry() -> OtelGuard {
    let env_filter = EnvFilter::from_default_env();

    let fmt_layer = tracing_subscriber::fmt::layer()
        .with_target(true)
        .with_thread_ids(false);

    let otel_endpoint = std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT").ok();

    if let Some(endpoint) = otel_endpoint {
        let service_name =
            std::env::var("OTEL_SERVICE_NAME").unwrap_or_else(|_| "realestate-backend".to_string());

        let resource = Resource::builder().with_service_name(service_name).build();

        let exporter = match SpanExporter::builder()
            .with_tonic()
            .with_endpoint(endpoint)
            .build()
        {
            Ok(e) => e,
            Err(e) => {
                eprintln!("Failed to create OTLP exporter, falling back to stdout: {e}");
                Registry::default().with(env_filter).with(fmt_layer).init();
                return OtelGuard { provider: None };
            }
        };

        let provider = SdkTracerProvider::builder()
            .with_resource(resource)
            .with_batch_exporter(exporter)
            .build();

        let tracer = provider.tracer("realestate-backend");

        let otel_layer = tracing_opentelemetry::layer().with_tracer(tracer);

        Registry::default()
            .with(env_filter)
            .with(fmt_layer)
            .with(otel_layer)
            .init();

        OtelGuard {
            provider: Some(provider),
        }
    } else {
        Registry::default().with(env_filter).with(fmt_layer).init();

        OtelGuard { provider: None }
    }
}

pub struct OtelGuard {
    provider: Option<SdkTracerProvider>,
}

impl Drop for OtelGuard {
    fn drop(&mut self) {
        if let Some(provider) = self.provider.take() {
            if let Err(e) = provider.shutdown() {
                eprintln!("Error shutting down OTel tracer provider: {e}");
            }
        }
    }
}
