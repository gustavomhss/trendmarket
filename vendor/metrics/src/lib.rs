use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::fmt::Write as _;
use std::hash::{Hash, Hasher};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

#[macro_export]
macro_rules! counter {
    ($name:expr $(, $label_key:expr => $label_value:expr )* $(,)?) => {{
        let labels = vec![$(($label_key.to_string(), format!("{}", $label_value))),*];
        $crate::counter_with_labels($name, labels)
    }};
}

#[macro_export]
macro_rules! histogram {
    ($name:expr $(, $label_key:expr => $label_value:expr )* $(,)?) => {{
        let labels = vec![$(($label_key.to_string(), format!("{}", $label_value))),*];
        $crate::histogram_with_labels($name, labels)
    }};
}

static REGISTRY: Lazy<Mutex<HashMap<MetricKey, MetricData>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

#[derive(Clone)]
pub struct Counter {
    value: Arc<AtomicU64>,
}

impl Counter {
    pub fn increment(&self, value: u64) {
        self.value.fetch_add(value, Ordering::Relaxed);
    }
}

#[derive(Clone)]
pub struct Histogram {
    inner: Arc<HistogramInner>,
}

impl Histogram {
    pub fn record(&self, value: f64) {
        let mut guard = self.inner.values.lock().expect("histogram lock poisoned");
        guard.push(value);
    }
}

struct HistogramInner {
    values: Mutex<Vec<f64>>,
}

impl HistogramInner {
    fn new() -> Self {
        Self {
            values: Mutex::new(Vec::new()),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum MetricType {
    Counter,
    Histogram,
}

#[derive(Clone)]
struct MetricKey {
    name: String,
    labels: Vec<(String, String)>,
    metric_type: MetricType,
}

impl PartialEq for MetricKey {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
            && self.metric_type == other.metric_type
            && self.labels == other.labels
    }
}

impl Eq for MetricKey {}

impl Hash for MetricKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.name.hash(state);
        self.metric_type.hash(state);
        for label in &self.labels {
            label.hash(state);
        }
    }
}

enum MetricData {
    Counter(Arc<AtomicU64>),
    Histogram(Arc<HistogramInner>),
}

pub struct MetricSnapshot {
    pub name: String,
    pub labels: Vec<(String, String)>,
    pub kind: SnapshotKind,
}

pub enum SnapshotKind {
    Counter(u64),
    Histogram { values: Vec<f64> },
}

pub fn counter_with_labels(name: &str, mut labels: Vec<(String, String)>) -> Counter {
    normalize_labels(&mut labels);
    let mut registry = REGISTRY.lock().expect("metrics registry poisoned");
    let key = MetricKey {
        name: name.to_string(),
        labels: labels.clone(),
        metric_type: MetricType::Counter,
    };
    match registry.entry(key.clone()) {
        std::collections::hash_map::Entry::Occupied(entry) => {
            if let MetricData::Counter(value) = entry.get() {
                Counter {
                    value: value.clone(),
                }
            } else {
                panic!("metric registered with different type");
            }
        }
        std::collections::hash_map::Entry::Vacant(entry) => {
            let value = Arc::new(AtomicU64::new(0));
            entry.insert(MetricData::Counter(value.clone()));
            Counter { value }
        }
    }
}

pub fn histogram_with_labels(name: &str, mut labels: Vec<(String, String)>) -> Histogram {
    normalize_labels(&mut labels);
    let mut registry = REGISTRY.lock().expect("metrics registry poisoned");
    let key = MetricKey {
        name: name.to_string(),
        labels: labels.clone(),
        metric_type: MetricType::Histogram,
    };
    match registry.entry(key.clone()) {
        std::collections::hash_map::Entry::Occupied(entry) => {
            if let MetricData::Histogram(inner) = entry.get() {
                Histogram {
                    inner: inner.clone(),
                }
            } else {
                panic!("metric registered with different type");
            }
        }
        std::collections::hash_map::Entry::Vacant(entry) => {
            let inner = Arc::new(HistogramInner::new());
            entry.insert(MetricData::Histogram(inner.clone()));
            Histogram { inner }
        }
    }
}

fn normalize_labels(labels: &mut Vec<(String, String)>) {
    labels.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)));
}

pub fn gather() -> Vec<MetricSnapshot> {
    let registry = REGISTRY.lock().expect("metrics registry poisoned");
    registry
        .iter()
        .map(|(key, data)| match data {
            MetricData::Counter(value) => MetricSnapshot {
                name: key.name.clone(),
                labels: key.labels.clone(),
                kind: SnapshotKind::Counter(value.load(Ordering::Relaxed)),
            },
            MetricData::Histogram(inner) => {
                let values = inner
                    .values
                    .lock()
                    .expect("histogram lock poisoned")
                    .clone();
                MetricSnapshot {
                    name: key.name.clone(),
                    labels: key.labels.clone(),
                    kind: SnapshotKind::Histogram { values },
                }
            }
        })
        .collect()
}

pub fn render_prometheus() -> String {
    let snapshots = gather();
    let mut buffer = String::new();
    for snapshot in snapshots {
        match snapshot.kind {
            SnapshotKind::Counter(value) => {
                let _ = writeln!(buffer, "# TYPE {} counter", snapshot.name);
                let labels = format_labels(&snapshot.labels);
                let _ = writeln!(buffer, "{}{} {}", snapshot.name, labels, value);
            }
            SnapshotKind::Histogram { values } => {
                let _ = writeln!(buffer, "# TYPE {} histogram", snapshot.name);
                let labels = format_labels(&snapshot.labels);
                let mut sorted = values.clone();
                sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
                let buckets = [1.0_f64, 5.0, 10.0, 25.0, 50.0];
                let mut counts = vec![0_u64; buckets.len()];
                for value in &sorted {
                    for (idx, bucket) in buckets.iter().enumerate() {
                        if value <= bucket {
                            counts[idx] += 1;
                        }
                    }
                }
                let mut cumulative = 0_u64;
                for (bucket, count) in buckets.iter().zip(counts.iter()) {
                    cumulative += count;
                    let mut bucket_labels = snapshot.labels.clone();
                    bucket_labels.push(("le".to_string(), format_bucket(*bucket)));
                    let bucket_fmt = format_labels(&bucket_labels);
                    let _ = writeln!(
                        buffer,
                        "{}_bucket{} {}",
                        snapshot.name, bucket_fmt, cumulative
                    );
                }
                let mut inf_labels = snapshot.labels.clone();
                inf_labels.push(("le".to_string(), "+Inf".to_string()));
                let inf_fmt = format_labels(&inf_labels);
                let count_total = sorted.len() as u64;
                let _ = writeln!(
                    buffer,
                    "{}_bucket{} {}",
                    snapshot.name, inf_fmt, count_total
                );
                let sum: f64 = sorted.iter().copied().sum();
                let _ = writeln!(buffer, "{}_sum{} {}", snapshot.name, labels, sum);
                let _ = writeln!(buffer, "{}_count{} {}", snapshot.name, labels, count_total);
            }
        }
    }
    buffer
}

fn format_labels(labels: &[(String, String)]) -> String {
    if labels.is_empty() {
        String::new()
    } else {
        let mut out = String::from("{");
        for (idx, (key, value)) in labels.iter().enumerate() {
            if idx > 0 {
                out.push(',');
            }
            out.push_str(key);
            out.push('=');
            out.push('"');
            out.push_str(&escape_label_value(value));
            out.push('"');
        }
        out.push('}');
        out
    }
}

fn escape_label_value(input: &str) -> String {
    let mut escaped = String::new();
    for ch in input.chars() {
        match ch {
            '\\' => escaped.push_str("\\\\"),
            '"' => escaped.push_str("\\\""),
            '\n' => escaped.push_str("\\n"),
            _ => escaped.push(ch),
        }
    }
    escaped
}

fn format_bucket(bucket: f64) -> String {
    if bucket.is_finite() {
        format!("{:.6}", bucket)
    } else {
        "+Inf".to_string()
    }
}

pub use Counter as CounterHandle;
pub use Histogram as HistogramHandle;
