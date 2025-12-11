//! Tracing and OpenTelemetry setup for floatctl CLI
//!
//! Provides unified tracing initialization with optional OTLP export.
//!
//! Usage:
//!   floatctl --debug ...              # Debug logging to console
//!   floatctl --otel ...               # Export traces to OTLP endpoint
//!   RUST_LOG=floatctl=debug floatctl  # Fine-grained log control
//!
//! Environment variables:
//!   RUST_LOG                          # Log filter (default: info)
//!   OTEL_EXPORTER_OTLP_ENDPOINT       # OTLP endpoint (default: http://localhost:4317)
//!   OTEL_SERVICE_NAME                 # Service name (default: floatctl)

use anyhow::{anyhow, Result};
use tracing_subscriber::EnvFilter;

/// Tracing configuration options
#[derive(Debug, Clone, Default)]
pub struct TracingConfig {
    /// Enable debug logging (sets RUST_LOG=debug if not already set)
    pub debug: bool,
    /// Enable OpenTelemetry OTLP export
    pub otel: bool,
}

/// Initialize tracing with console output only (no OTEL)
pub fn init_tracing(config: &TracingConfig) -> Result<()> {
    let filter = if config.debug {
        // Debug mode: set debug level unless RUST_LOG is explicitly set
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("debug"))
    } else {
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"))
    };

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(config.debug) // Show targets in debug mode
        .compact()
        .try_init()
        .map_err(|err| anyhow!(err))
}

/// Initialize tracing with OpenTelemetry OTLP export
#[cfg(feature = "telemetry")]
pub fn init_tracing_with_otel(config: &TracingConfig) -> Result<()> {
    use opentelemetry::trace::TracerProvider as _;
    use opentelemetry::KeyValue;
    use opentelemetry_otlp::WithExportConfig;
    use opentelemetry_sdk::trace::TracerProvider;
    use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

    let endpoint = std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT")
        .unwrap_or_else(|_| "http://localhost:4317".to_string());

    let service_name = std::env::var("OTEL_SERVICE_NAME").unwrap_or_else(|_| "floatctl".to_string());

    // Build OTLP exporter
    let otlp_exporter = opentelemetry_otlp::SpanExporter::builder()
        .with_tonic()
        .with_endpoint(&endpoint)
        .build()
        .map_err(|e| anyhow!("Failed to create OTLP exporter: {}", e))?;

    // Build resource with service name
    let resource = opentelemetry_sdk::Resource::new(vec![
        KeyValue::new("service.name", service_name.clone()),
    ]);

    // Build tracer provider with batch export
    let provider = TracerProvider::builder()
        .with_batch_exporter(otlp_exporter, opentelemetry_sdk::runtime::Tokio)
        .with_resource(resource)
        .build();

    let tracer = provider.tracer("floatctl");
    let telemetry_layer = tracing_opentelemetry::layer().with_tracer(tracer);

    // Store provider globally so it doesn't get dropped
    // (dropping would stop trace export)
    let _ = opentelemetry::global::set_tracer_provider(provider);

    let filter = if config.debug {
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("debug"))
    } else {
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"))
    };

    let fmt_layer = tracing_subscriber::fmt::layer()
        .with_target(config.debug)
        .compact();

    tracing_subscriber::registry()
        .with(filter)
        .with(fmt_layer)
        .with(telemetry_layer)
        .init();

    tracing::info!(
        endpoint = %endpoint,
        service = %service_name,
        "OpenTelemetry tracing initialized"
    );

    Ok(())
}

/// Shutdown OpenTelemetry (flush pending spans)
#[cfg(feature = "telemetry")]
pub fn shutdown_otel() {
    opentelemetry::global::shutdown_tracer_provider();
}

/// No-op shutdown when telemetry is disabled
#[cfg(not(feature = "telemetry"))]
pub fn shutdown_otel() {}

/// Initialize tracing based on configuration
///
/// Chooses between console-only and OTEL based on config.otel flag
pub fn init(config: &TracingConfig) -> Result<()> {
    #[cfg(feature = "telemetry")]
    if config.otel {
        return init_tracing_with_otel(config);
    }

    init_tracing(config)
}
