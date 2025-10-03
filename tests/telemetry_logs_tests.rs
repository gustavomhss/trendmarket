use std::io::{self, Write};
use std::sync::{Arc, Mutex};

use credit_engine_core::telemetry_logs::{json_layer, level_filter, LogConfig};
use opentelemetry::trace::{TraceContextExt, Tracer, TracerProvider as _};
use opentelemetry::Context;
use opentelemetry_sdk::trace::SdkTracerProvider;
use serde_json::Value;
use tracing::subscriber::with_default;
use tracing_opentelemetry::OpenTelemetrySpanExt;
use tracing_subscriber::fmt::MakeWriter;
use tracing_subscriber::layer::{Layer as _, SubscriberExt};
use tracing_subscriber::Registry;

#[derive(Clone)]
struct BufferMakeWriter {
    buffer: Arc<Mutex<Vec<u8>>>,
}

impl BufferMakeWriter {
    fn new(buffer: Arc<Mutex<Vec<u8>>>) -> Self {
        Self { buffer }
    }
}

impl<'a> MakeWriter<'a> for BufferMakeWriter {
    type Writer = BufferWriter;

    fn make_writer(&'a self) -> Self::Writer {
        BufferWriter {
            buffer: self.buffer.clone(),
        }
    }
}

struct BufferWriter {
    buffer: Arc<Mutex<Vec<u8>>>,
}

impl Write for BufferWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let mut guard = self.buffer.lock().expect("buffer poisoned");
        guard.extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

fn base_config() -> LogConfig {
    LogConfig {
        level: "info".to_string(),
        service: "ce-amm".to_string(),
        env: "dev".to_string(),
        version: "1.0.0".to_string(),
    }
}

fn read_single_line(buffer: &Arc<Mutex<Vec<u8>>>) -> String {
    let data = buffer.lock().expect("buffer poisoned").clone();
    let output = String::from_utf8(data).expect("invalid utf8");
    output
        .lines()
        .last()
        .expect("expected at least one line")
        .to_string()
}

#[test]
fn emits_minimal_json_line() {
    let cfg = base_config();
    let buffer = Arc::new(Mutex::new(Vec::new()));
    let make_writer = BufferMakeWriter::new(buffer.clone());

    let layer = json_layer(&cfg)
        .expect("layer")
        .with_writer(make_writer.clone());
    let subscriber = Registry::default()
        .with(layer)
        .with(level_filter(&cfg.level).expect("level"));

    with_default(subscriber, || {
        tracing::info!("swap executed");
    });

    let line = read_single_line(&buffer);
    let json: Value = serde_json::from_str(&line).expect("valid json");

    assert!(json.get("ts").is_some());
    assert_eq!(json["level"], "info");
    assert_eq!(json["msg"], "swap executed");
    assert_eq!(json["service"], "ce-amm");
    assert_eq!(json["env"], "dev");
    assert_eq!(json["version"], "1.0.0");
}

#[test]
fn includes_trace_and_span_ids_with_op_when_context_available() {
    let cfg = base_config();
    let buffer = Arc::new(Mutex::new(Vec::new()));
    let make_writer = BufferMakeWriter::new(buffer.clone());

    let layer = json_layer(&cfg)
        .expect("layer")
        .with_writer(make_writer.clone());
    let provider = SdkTracerProvider::builder().build();
    let tracer = provider.tracer("obs1-tests");
    let otel_layer = tracing_opentelemetry::layer().with_tracer(tracer.clone());

    let composite_layer = layer.and_then(otel_layer);

    let subscriber = Registry::default()
        .with(composite_layer)
        .with(level_filter("info").expect("level"));

    let guard = tracing::subscriber::set_default(subscriber);
    {
        let otel_span = tracer.start("swap-root");
        let otel_context = Context::current_with_span(otel_span);
        let span = tracing::info_span!("swap_span", op = "swap");
        span.set_parent(otel_context);
        let _entered = span.enter();
        tracing::info!(price = 10.5, "swap complete");
    }
    drop(guard);

    let line = read_single_line(&buffer);
    let json: Value = serde_json::from_str(&line).expect("valid json");

    assert_eq!(json["op"], "swap");
    assert!(json.get("trace_id").is_some(), "trace_id missing");
    assert!(json.get("span_id").is_some(), "span_id missing");
}

#[test]
fn blocks_common_pii_fields() {
    let cfg = base_config();
    let buffer = Arc::new(Mutex::new(Vec::new()));
    let make_writer = BufferMakeWriter::new(buffer.clone());

    let layer = json_layer(&cfg)
        .expect("layer")
        .with_writer(make_writer.clone());
    let subscriber = Registry::default()
        .with(layer)
        .with(level_filter("info").expect("level"));

    with_default(subscriber, || {
        tracing::info!(
            email = "cliente@exemplo.com",
            cpf = "00011122233",
            "customer event"
        );
    });

    let line = read_single_line(&buffer);
    let json: Value = serde_json::from_str(&line).expect("valid json");

    assert!(json.get("email").is_none());
    assert!(json.get("cpf").is_none());
    assert_eq!(json["msg"], "customer event");
}
