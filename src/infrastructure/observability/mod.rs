use crate::config::Config;
use opentelemetry::{global, KeyValue};
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::{runtime, trace as sdktrace, Resource};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, Registry};

pub struct ObservabilityGuard;

impl Drop for ObservabilityGuard {
    fn drop(&mut self) {
        shutdown();
    }
}

pub fn init(config: &Config) -> Result<ObservabilityGuard, Box<dyn std::error::Error>> {
    init_tracing(config)?;
    init_metrics(config)?;
    Ok(ObservabilityGuard)
}

fn init_tracing(config: &Config) -> Result<(), Box<dyn std::error::Error>> {
    // Standard formatting layer (STDOUT)
    let fmt_layer = tracing_subscriber::fmt::layer()
        .with_thread_ids(true)
        .with_target(true);

    let env_filter =
        tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "debug".into());

    // Optional OTLP layer for distributed tracing
    if let Some(endpoint) = &config.otel_exporter_endpoint {
        let exporter = opentelemetry_otlp::new_exporter()
            .tonic()
            .with_endpoint(endpoint);

        let tracer = opentelemetry_otlp::new_pipeline()
            .tracing()
            .with_exporter(exporter)
            .with_trace_config(sdktrace::config().with_resource(Resource::new(vec![
                KeyValue::new("service.name", config.service_name.clone()),
            ])))
            .install_batch(runtime::Tokio)?;

        let otel_layer = tracing_opentelemetry::layer().with_tracer(tracer);

        Registry::default()
            .with(env_filter)
            .with(fmt_layer)
            .with(otel_layer)
            .init();
    } else {
        Registry::default().with(env_filter).with(fmt_layer).init();
    }

    Ok(())
}

fn init_metrics(config: &Config) -> Result<(), Box<dyn std::error::Error>> {
    metrics_exporter_prometheus::PrometheusBuilder::new()
        .with_http_listener(([0, 0, 0, 0], config.metrics_port))
        .install()?;

    tracing::info!(
        "Metrics exporter (Prometheus) started on port {}",
        config.metrics_port
    );
    Ok(())
}

pub fn shutdown() {
    global::shutdown_tracer_provider();
}
