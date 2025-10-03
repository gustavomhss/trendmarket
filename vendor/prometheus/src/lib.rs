use std::fmt;
use std::sync::{Arc, Mutex};

pub mod proto {
    #[derive(Clone, Debug, Default, PartialEq)]
    pub struct LabelPair {
        pub name: String,
        pub value: String,
    }

    #[derive(Clone, Debug, Default, PartialEq)]
    pub struct Counter {
        pub value: f64,
    }

    #[derive(Clone, Debug, Default, PartialEq)]
    pub struct Bucket {
        pub cumulative_count: u64,
        pub upper_bound: f64,
    }

    #[derive(Clone, Debug, Default, PartialEq)]
    pub struct Histogram {
        pub sample_count: u64,
        pub sample_sum: f64,
        pub bucket: Vec<Bucket>,
    }

    #[derive(Clone, Debug, Default, PartialEq)]
    pub struct Metric {
        pub label: Vec<LabelPair>,
        pub counter: Option<Counter>,
        pub histogram: Option<Histogram>,
    }

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum MetricType {
        Counter,
        Histogram,
    }

    impl Default for MetricType {
        fn default() -> Self {
            MetricType::Counter
        }
    }

    #[derive(Clone, Debug, Default, PartialEq)]
    pub struct MetricFamily {
        pub name: String,
        pub help: String,
        pub r#type: MetricType,
        pub metric: Vec<Metric>,
    }
}

pub mod core {
    use super::proto::MetricFamily;

    pub trait Collector: Send + Sync {
        fn collect(&self) -> Vec<MetricFamily>;
    }
}

#[derive(Debug)]
pub enum Error {
    Register(String),
    Poison(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Register(msg) | Error::Poison(msg) => f.write_str(msg),
        }
    }
}

impl std::error::Error for Error {}

#[derive(Clone, Default)]
pub struct Registry {
    inner: Arc<RegistryInner>,
}

#[derive(Default)]
struct RegistryInner {
    collectors: Mutex<Vec<Arc<dyn core::Collector>>>,
}

impl Registry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(
        &self,
        collector: Box<dyn core::Collector>,
    ) -> Result<(), Error> {
        let mut guard = self
            .inner
            .collectors
            .lock()
            .map_err(|err| Error::Poison(err.to_string()))?;
        guard.push(Arc::from(collector));
        Ok(())
    }

    pub fn gather(&self) -> Vec<proto::MetricFamily> {
        let guard = match self.inner.collectors.lock() {
            Ok(guard) => guard,
            Err(_) => return Vec::new(),
        };

        let mut families = Vec::new();
        for collector in guard.iter() {
            families.extend(collector.collect());
        }
        families
    }
}

pub fn gather(registry: &Registry) -> Vec<proto::MetricFamily> {
    registry.gather()
}

pub trait Encoder {
    fn encode(
        &self,
        metric_families: &[proto::MetricFamily],
        writer: &mut Vec<u8>,
    ) -> Result<(), std::io::Error>;
}

pub struct TextEncoder;

impl TextEncoder {
    pub fn new() -> Self {
        Self
    }
}

impl Encoder for TextEncoder {
    fn encode(
        &self,
        metric_families: &[proto::MetricFamily],
        writer: &mut Vec<u8>,
    ) -> Result<(), std::io::Error> {
        encode_metric_families(metric_families, writer)
    }
}

impl TextEncoder {
    pub fn encode(
        &self,
        metric_families: &[proto::MetricFamily],
        writer: &mut Vec<u8>,
    ) -> Result<(), std::io::Error> {
        encode_metric_families(metric_families, writer)
    }
}

fn encode_metric_families(
    metric_families: &[proto::MetricFamily],
    writer: &mut Vec<u8>,
) -> Result<(), std::io::Error> {
    use std::fmt::Write as _;

    let mut buffer = String::new();
    for family in metric_families {
        if !family.help.is_empty() {
            writeln!(&mut buffer, "# HELP {} {}", family.name, family.help)
                .map_err(io_error)?;
        }
        let metric_type = match family.r#type {
            proto::MetricType::Counter => "counter",
            proto::MetricType::Histogram => "histogram",
        };
        writeln!(&mut buffer, "# TYPE {} {}", family.name, metric_type).map_err(io_error)?;

        for metric in &family.metric {
            if let Some(counter) = &metric.counter {
                let labels = format_labels(&metric.label);
                writeln!(
                    &mut buffer,
                    "{}{} {}",
                    family.name,
                    labels,
                    counter.value
                )
                .map_err(io_error)?;
            }

            if let Some(histogram) = &metric.histogram {
                for bucket in &histogram.bucket {
                    let mut labels = metric.label.clone();
                    labels.push(proto::LabelPair {
                        name: "le".to_string(),
                        value: format_upper_bound(bucket.upper_bound),
                    });
                    let labels_formatted = format_labels(&labels);
                    writeln!(
                        &mut buffer,
                        "{}_bucket{} {}",
                        family.name,
                        labels_formatted,
                        bucket.cumulative_count
                    )
                    .map_err(io_error)?;
                }
                let mut labels = metric.label.clone();
                labels.push(proto::LabelPair {
                    name: "le".to_string(),
                    value: "+Inf".to_string(),
                });
                let labels_formatted = format_labels(&labels);
                writeln!(
                    &mut buffer,
                    "{}_bucket{} {}",
                    family.name,
                    labels_formatted,
                    histogram.sample_count
                )
                .map_err(io_error)?;

                let base_labels = format_labels(&metric.label);
                writeln!(
                    &mut buffer,
                    "{}_sum{} {}",
                    family.name,
                    base_labels,
                    histogram.sample_sum
                )
                .map_err(io_error)?;
                writeln!(
                    &mut buffer,
                    "{}_count{} {}",
                    family.name,
                    base_labels,
                    histogram.sample_count
                )
                .map_err(io_error)?;
            }
        }
    }

    writer.extend_from_slice(buffer.as_bytes());
    Ok(())
}

fn io_error<E: std::fmt::Display>(err: E) -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::Other, err.to_string())
}

fn format_labels(labels: &[proto::LabelPair]) -> String {
    if labels.is_empty() {
        String::new()
    } else {
        let mut result = String::from("{");
        for (index, label) in labels.iter().enumerate() {
            if index > 0 {
                result.push(',');
            }
            result.push_str(&label.name);
            result.push('=');
            result.push('"');
            result.push_str(&escape_label_value(&label.value));
            result.push('"');
        }
        result.push('}');
        result
    }
}

fn escape_label_value(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('\n', "\\n")
        .replace('"', "\\\"")
}

fn format_upper_bound(bound: f64) -> String {
    if bound.is_infinite() {
        "+Inf".to_string()
    } else if bound.is_nan() {
        "NaN".to_string()
    } else {
        bound.to_string()
    }
}
