use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Duration;

use opentelemetry::KeyValue;
use opentelemetry_sdk::metrics::data::{self, Metric as OtelMetric, ResourceMetrics};
use opentelemetry_sdk::metrics::{InMemoryMetricExporter, MetricError, PeriodicReader, SdkMeterProvider};
use prometheus::core::Collector;
use prometheus::proto::{Bucket, Counter, Histogram, LabelPair, Metric, MetricFamily, MetricType};
use prometheus::Registry;

#[derive(Clone)]
pub struct PrometheusExporter {
    registry: Registry,
    provider: Arc<SdkMeterProvider>,
}

impl PrometheusExporter {
    pub fn registry(&self) -> &Registry {
        &self.registry
    }

    pub fn provider(&self) -> Arc<SdkMeterProvider> {
        Arc::clone(&self.provider)
    }
}

#[derive(Default)]
pub struct PrometheusExporterBuilder {
    registry: Option<Registry>,
    interval: Duration,
}

pub fn exporter() -> PrometheusExporterBuilder {
    PrometheusExporterBuilder {
        registry: None,
        interval: Duration::from_secs(5),
    }
}

impl PrometheusExporterBuilder {
    pub fn with_registry(mut self, registry: Registry) -> Self {
        self.registry = Some(registry);
        self
    }

    pub fn with_collect_interval(mut self, interval: Duration) -> Self {
        self.interval = interval;
        self
    }

    pub fn build(self) -> Result<PrometheusExporter, MetricError> {
        let registry = self.registry.unwrap_or_else(Registry::new);
        let exporter = InMemoryMetricExporter::default();
        let reader = PeriodicReader::builder(exporter.clone())
            .with_interval(self.interval)
            .build();
        let provider = Arc::new(SdkMeterProvider::builder().with_reader(reader).build());

        let collector = OtelCollector {
            provider: Arc::clone(&provider),
            exporter: exporter.clone(),
        };
        registry
            .register(Box::new(collector))
            .map_err(|err| MetricError::Other(err.to_string()))?;

        Ok(PrometheusExporter {
            registry,
            provider,
        })
    }
}

struct OtelCollector {
    provider: Arc<SdkMeterProvider>,
    exporter: InMemoryMetricExporter,
}

impl Collector for OtelCollector {
    fn collect(&self) -> Vec<MetricFamily> {
        if self.provider.force_flush().is_err() {
            return Vec::new();
        }
        let metrics = match self.exporter.get_finished_metrics() {
            Ok(metrics) => metrics,
            Err(_) => Vec::new(),
        };
        self.exporter.reset();
        convert_resource_metrics(metrics)
    }
}

fn convert_resource_metrics(resource_metrics: Vec<ResourceMetrics>) -> Vec<MetricFamily> {
    let mut families: BTreeMap<String, MetricFamily> = BTreeMap::new();

    for rm in resource_metrics {
        for scope_metrics in rm.scope_metrics {
            for metric in scope_metrics.metrics {
                if let Some(mut family) = convert_metric(&metric) {
                    let entry = families
                        .entry(family.name.clone())
                        .or_insert_with(|| MetricFamily {
                            name: family.name.clone(),
                            help: family.help.clone(),
                            r#type: family.r#type,
                            metric: Vec::new(),
                        });
                    entry.metric.extend(family.metric.drain(..));
                }
            }
        }
    }

    families.into_values().collect()
}

fn convert_metric(metric: &OtelMetric) -> Option<MetricFamily> {
    let name = metric.name.to_string();
    let description = metric.description.to_string();
    let data = metric.data.as_any();

    if let Some(sum) = data.downcast_ref::<data::Sum<u64>>() {
        return Some(build_sum_family(&name, &description, &sum.data_points, |v| v as f64));
    }

    if let Some(sum) = data.downcast_ref::<data::Sum<f64>>() {
        return Some(build_sum_family(&name, &description, &sum.data_points, |v| v));
    }

    if let Some(histogram) = data.downcast_ref::<data::Histogram<f64>>() {
        return Some(build_histogram_family(&name, &description, histogram));
    }

    None
}

fn build_sum_family<T, F>(
    name: &str,
    description: &str,
    data_points: &[data::SumDataPoint<T>],
    to_f64: F,
) -> MetricFamily
where
    T: Copy,
    F: Fn(T) -> f64,
{
    let mut metrics = Vec::with_capacity(data_points.len());
    for point in data_points {
        let mut metric = Metric::default();
        metric.label = convert_labels(&point.attributes);
        metric.counter = Some(Counter {
            value: to_f64(point.value),
        });
        metrics.push(metric);
    }

    MetricFamily {
        name: name.to_string(),
        help: description.to_string(),
        r#type: MetricType::Counter,
        metric: metrics,
    }
}

fn build_histogram_family(
    name: &str,
    description: &str,
    histogram: &data::Histogram<f64>,
) -> MetricFamily {
    let mut metrics = Vec::with_capacity(histogram.data_points.len());

    for point in &histogram.data_points {
        let mut metric = Metric::default();
        metric.label = convert_labels(&point.attributes);

        let mut buckets = Vec::new();
        let mut cumulative = 0u64;
        for (idx, bound) in point.bounds.iter().enumerate() {
            let count = *point.bucket_counts.get(idx).unwrap_or(&0);
            cumulative += count;
            buckets.push(Bucket {
                cumulative_count: cumulative,
                upper_bound: *bound,
            });
        }

        let proto_histogram = Histogram {
            sample_count: point.count,
            sample_sum: point.sum,
            bucket: buckets,
        };
        metric.histogram = Some(proto_histogram);
        metrics.push(metric);
    }

    MetricFamily {
        name: name.to_string(),
        help: description.to_string(),
        r#type: MetricType::Histogram,
        metric: metrics,
    }
}

fn convert_labels(attributes: &[KeyValue]) -> Vec<LabelPair> {
    attributes
        .iter()
        .map(|kv| LabelPair {
            name: kv.key.to_string(),
            value: kv.value.to_string(),
        })
        .collect()
}
