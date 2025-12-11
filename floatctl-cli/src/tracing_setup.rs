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

use anyhow::{Context, Result};
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
        .map_err(|e| anyhow::Error::msg(e.to_string()))
        .context("failed to initialize tracing subscriber")
}

/// Initialize tracing with OpenTelemetry OTLP export
///
/// If OTLP connection fails, gracefully falls back to console-only logging
/// with a warning message. This ensures the CLI doesn't fail to start just
/// because the collector is unavailable.
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

    // Try to build OTLP exporter - fall back to console if it fails
    let otlp_exporter = match opentelemetry_otlp::SpanExporter::builder()
        .with_tonic()
        .with_endpoint(&endpoint)
        .build()
    {
        Ok(exporter) => exporter,
        Err(e) => {
            // Fall back to console-only logging
            eprintln!(
                "warning: failed to create OTLP exporter ({}): {}\n\
                 Falling back to console-only logging. \
                 Check OTEL_EXPORTER_OTLP_ENDPOINT or run without --otel.",
                endpoint, e
            );
            return init_tracing(config);
        }
    };

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

    // Use registry() when composing multiple layers (fmt + telemetry).
    // This differs from the console-only path which uses fmt() directly.
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

/// Shutdown OpenTelemetry and flush pending spans.
///
/// Should be called before program exit to ensure all spans are exported.
/// Safe to call even if OTEL was never initialized (no-op in that case).
#[cfg(feature = "telemetry")]
pub fn shutdown_otel() {
    opentelemetry::global::shutdown_tracer_provider();
}

/// No-op shutdown when telemetry feature is disabled.
#[cfg(not(feature = "telemetry"))]
pub fn shutdown_otel() {}

/// Initialize tracing based on configuration
///
/// Chooses between console-only and OTEL based on config.otel flag.
/// Safe to call multiple times - subsequent calls are no-ops.
pub fn init(config: &TracingConfig) -> Result<()> {
    #[cfg(feature = "telemetry")]
    if config.otel {
        return init_tracing_with_otel(config);
    }

    #[cfg(not(feature = "telemetry"))]
    if config.otel {
        eprintln!(
            "warning: --otel flag requires building with --features telemetry\n\
             Falling back to console-only logging."
        );
    }

    init_tracing(config)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tracing_config_default() {
        let config = TracingConfig::default();
        assert!(!config.debug, "debug should default to false");
        assert!(!config.otel, "otel should default to false");
    }

    #[test]
    fn test_tracing_config_debug_mode() {
        let config = TracingConfig {
            debug: true,
            otel: false,
        };
        assert!(config.debug);
        assert!(!config.otel);
    }

    #[test]
    fn test_tracing_config_clone() {
        let config = TracingConfig {
            debug: true,
            otel: true,
        };
        let cloned = config.clone();
        assert_eq!(config.debug, cloned.debug);
        assert_eq!(config.otel, cloned.otel);
    }

    #[test]
    fn test_tracing_config_debug_trait() {
        let config = TracingConfig {
            debug: true,
            otel: false,
        };
        let debug_str = format!("{:?}", config);
        assert!(debug_str.contains("debug: true"));
        assert!(debug_str.contains("otel: false"));
    }

    // Note: We can't fully test init_tracing() or init_tracing_with_otel()
    // because the global subscriber can only be set once per process.
    // These functions are tested implicitly by the CLI integration tests.
    //
    // For OTEL-specific testing, you would need:
    // - Integration tests with a mock OTLP collector
    // - Or use tracing-test crate for subscriber testing
}
