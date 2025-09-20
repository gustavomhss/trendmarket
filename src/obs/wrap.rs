use once_cell::sync::OnceCell;
use opentelemetry::{global, KeyValue};
use opentelemetry::metrics::Histogram;
use std::time::Instant;

static HIST: OnceCell<Histogram<f64>> = OnceCell::new();

fn histogram() -> Histogram<f64> {
    HIST.get_or_init(|| {
        let meter = global::meter("obs.wrap");
        meter.f64_histogram("op_duration_seconds").with_description("operation duration").init()
    }).clone()
}

pub fn time<F, T>(op: &str, f: F) -> T
where
    F: FnOnce() -> T,
{
    let start = Instant::now();
    let out = f();
    let sec = start.elapsed().as_secs_f64();
    histogram().record(sec, &[KeyValue::new("op", op.to_string())]);
    out
}
