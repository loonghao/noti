//! OpenTelemetry tracing initialization with OTLP exporter support.
//!
//! When `NOTI_OTEL_ENDPOINT` is set, spans are exported to the configured
//! OTLP collector (e.g. Jaeger, Tempo, Honeycomb). When unset, the no-op
//! tracer is used and no spans are exported — zero overhead in local dev.
//!
//! Environment variables:
//! | Variable | Default | Description |
//! |---|---|---|
//! | `NOTI_OTEL_ENDPOINT` | *(empty)* | OTLP collector gRPC endpoint (e.g. `http://localhost:4317`). When empty, tracing is disabled. |
//! | `NOTI_OTEL_SERVICE_NAME` | `noti-server` | Service name used in OTEL resource and span names. |

use std::env;
use std::time::Duration;

use opentelemetry::trace::TracerProvider as _;
use opentelemetry::KeyValue;
use opentelemetry_otlp::WithExportConfig;
use tracing_subscriber::layer::SubscriberExt as _;
use tracing_subscriber::util::SubscriberInitExt as _;

/// Configuration for OpenTelemetry tracing, sourced from environment variables.
#[derive(Debug, Clone)]
pub struct OtelConfig {
    /// OTLP collector gRPC endpoint (e.g. `http://localhost:4317`).
    /// When `None`, OpenTelemetry is disabled (no-op tracer).
    pub endpoint: Option<String>,
    /// Service name reported as the `service.name` OTEL resource attribute.
    pub service_name: String,
}

impl Default for OtelConfig {
    fn default() -> Self {
        Self {
            endpoint: env::var("NOTI_OTEL_ENDPOINT").ok().filter(|s| !s.is_empty()),
            service_name: env::var("NOTI_OTEL_SERVICE_NAME")
                .unwrap_or_else(|_| "noti-server".to_string()),
        }
    }
}

impl OtelConfig {
    /// Whether OTEL tracing is enabled (endpoint is configured).
    #[must_use]
    pub fn is_enabled(&self) -> bool {
        self.endpoint.is_some()
    }
}

/// Initialize OpenTelemetry with an explicit config.
pub fn init_otel_with_config(config: &OtelConfig) -> Option<OtelGuard> {
    let endpoint = config.endpoint.as_ref()?;

    // Build the OTLP exporter with gRPC (tonic) transport
    let exporter = opentelemetry_otlp::SpanExporter::builder()
        .with_tonic()
        .with_endpoint(endpoint.as_str())
        .with_timeout(Duration::from_secs(10))
        .build()
        .expect("OTLP span exporter must be buildable");

    // Batch processor with Tokio runtime for async export
    let processor = opentelemetry_sdk::trace::BatchSpanProcessor::builder(
        exporter,
        opentelemetry_sdk::runtime::Tokio,
    )
    .build();

    // Resource with service identity attributes
    let resource = opentelemetry_sdk::Resource::new([
        KeyValue::new("service.name", config.service_name.clone()),
        KeyValue::new("service.version", env!("CARGO_PKG_VERSION")),
    ]);

    // Build the tracer provider
    let tracer_provider = opentelemetry_sdk::trace::TracerProvider::builder()
        .with_resource(resource)
        .with_span_processor(processor)
        .build();

    let tracer = tracer_provider.tracer("noti-server");

    // Create the OpenTelemetry tracing layer and register it as the global subscriber.
    // Using `try_init` so that if a subscriber is already set (e.g. in tests) this is a no-op.
    let otel_layer = tracing_opentelemetry::layer().with_tracer(tracer);
    let _ = tracing_subscriber::registry().with(otel_layer).try_init();

    // Install the OTEL tracer provider as the global default
    opentelemetry::global::set_tracer_provider(tracer_provider.clone());

    tracing::info!(
        endpoint = %endpoint,
        service_name = %config.service_name,
        "OpenTelemetry tracing enabled (OTLP)"
    );

    Some(OtelGuard {
        tracer_provider,
    })
}

/// Initialize OpenTelemetry tracing with OTLP exporter.
///
/// Returns `None` if `NOTI_OTEL_ENDPOINT` is not set (OTEL disabled).
///
/// The returned [`OtelGuard`] must be kept alive for the duration of the
/// program. Dropping it triggers an orderly flush of pending spans.
pub fn init_otel() -> Option<OtelGuard> {
    let config = OtelConfig::default();
    init_otel_with_config(&config)
}

/// Guard that flushes OpenTelemetry spans on drop.
pub struct OtelGuard {
    tracer_provider: opentelemetry_sdk::trace::TracerProvider,
}

impl OtelGuard {
    /// Force-flush any pending spans and shut down the tracer provider.
    pub fn shutdown(&self) {
        let _ = self.tracer_provider.shutdown();
    }
}

impl Drop for OtelGuard {
    fn drop(&mut self) {
        self.shutdown();
    }
}
