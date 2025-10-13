use credit_engine_core::obs::staleness::{StalenessCfg, StalenessRegistry, SourceKey};
use opentelemetry::metrics::MeterProvider as _;
use opentelemetry_sdk::metrics::SdkMeterProvider;
use opentelemetry_sdk::Resource;

const NOW_OVERRIDE_ENV: &str = "STALENESS_NOW_OVERRIDE";

fn build_registry(enable_recency: bool) -> StalenessRegistry {
    let provider = SdkMeterProvider::builder()
        .with_resource(Resource::builder().build())
        .build();
    let meter = provider.meter("staleness-tests");
    StalenessRegistry::new(&meter, StalenessCfg { enable_recency })
}

fn find_metric(
    snapshot: Vec<(SourceKey, f64, Option<f64>)>,
    source: &str,
    domain: &str,
) -> Option<(f64, Option<f64>)> {
    for (key, freshness, recency) in snapshot {
        if key.source == source && key.domain == domain {
            return Some((freshness, recency));
        }
    }
    None
}

#[test]
fn clamp_zero_on_clock_step() {
    std::env::set_var(NOW_OVERRIDE_ENV, "200");
    let registry = build_registry(false);
    registry.update_arrival("alpha", "credit");
    std::env::remove_var(NOW_OVERRIDE_ENV);
    let entry = find_metric(registry.metrics_snapshot(150), "alpha", "credit");
    if let Some((freshness, _)) = entry {
        assert_eq!(freshness, 0.0);
    } else {
        panic!("missing freshness entry");
    }
}

#[test]
fn monotonic_between_arrivals() {
    std::env::set_var(NOW_OVERRIDE_ENV, "100");
    let registry = build_registry(false);
    registry.update_arrival("alpha", "credit");
    std::env::remove_var(NOW_OVERRIDE_ENV);
    let mut previous = 0.0;
    for now in [101_u64, 120, 150, 190, 200, 240] {
        let entry = find_metric(registry.metrics_snapshot(now), "alpha", "credit");
        if let Some((freshness, _)) = entry {
            assert!(freshness >= previous);
            previous = freshness;
        } else {
            panic!("missing freshness entry");
        }
    }
}

#[test]
fn reset_on_arrival() {
    std::env::set_var(NOW_OVERRIDE_ENV, "50");
    let registry = build_registry(false);
    registry.update_arrival("alpha", "credit");
    std::env::set_var(NOW_OVERRIDE_ENV, "100");
    registry.update_arrival("alpha", "credit");
    std::env::remove_var(NOW_OVERRIDE_ENV);
    let entry = find_metric(registry.metrics_snapshot(101), "alpha", "credit");
    if let Some((freshness, _)) = entry {
        assert!(freshness <= 1.0);
    } else {
        panic!("missing freshness entry");
    }
}

#[test]
fn optional_recency() {
    std::env::set_var(NOW_OVERRIDE_ENV, "40");
    let registry = build_registry(true);
    registry.update_origin("alpha", "credit", 30);
    std::env::set_var(NOW_OVERRIDE_ENV, "60");
    registry.update_arrival("alpha", "credit");
    std::env::remove_var(NOW_OVERRIDE_ENV);
    let entry = find_metric(registry.metrics_snapshot(90), "alpha", "credit");
    if let Some((_, recency)) = entry {
        if let Some(value) = recency {
            assert!(value >= 0.0);
        } else {
            panic!("missing recency value");
        }
    } else {
        panic!("missing freshness entry");
    }
}
