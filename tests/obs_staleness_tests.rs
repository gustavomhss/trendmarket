use credit_engine_core::obs::consumers::{
    handle_cdc_orders, handle_chain_header, handle_market_feed, handle_oracle_event, CdcOrderEvent,
    ChainHeader, HandlerError, MarketFeedEvent, OracleEvent, OrderChange,
};
use credit_engine_core::obs::staleness::{global_staleness_registry, StalenessSample};
use once_cell::sync::Lazy;
use std::sync::Mutex;
use std::time::{Duration, SystemTime};

static TEST_GUARD: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

fn find_sample<'a>(samples: &'a [StalenessSample], source: &str) -> Option<&'a StalenessSample> {
    samples.iter().find(|sample| sample.source == source)
}

#[test]
fn oracle_handler_updates_freshness() -> Result<(), HandlerError> {
    let _guard = TEST_GUARD.lock().unwrap();
    let registry = global_staleness_registry();
    registry.clear();
    let event = OracleEvent {
        instrument: "EURUSD".to_string(),
        best_bid: 1.0,
        best_ask: 1.0005,
        origin_timestamp: Some(SystemTime::now() - Duration::from_secs(1)),
    };
    handle_oracle_event(event, |_| Ok(()))?;
    let samples = registry.snapshot();
    let oracle = find_sample(&samples, "oracle").expect("oracle sample");
    assert!(oracle.domain == "pricing");
    assert!(oracle.freshness_seconds >= 0.0);
    Ok(())
}

#[test]
fn market_feed_without_origin_uses_arrival() -> Result<(), HandlerError> {
    let _guard = TEST_GUARD.lock().unwrap();
    let registry = global_staleness_registry();
    registry.clear();
    let event = MarketFeedEvent {
        venue: "XNAS".to_string(),
        symbol: "AAPL".to_string(),
        last_price: 180.0,
        size: 25.0,
        sequence: 42,
        exchange_timestamp: None,
    };
    handle_market_feed(event, |_| Ok(()))?;
    let samples = registry.snapshot();
    let market = find_sample(&samples, "market_feed").expect("market sample");
    assert_eq!(market.domain, "market");
    assert!(market.freshness_seconds >= 0.0);
    Ok(())
}

#[test]
fn cdc_orders_records_commit_timestamp() -> Result<(), HandlerError> {
    let _guard = TEST_GUARD.lock().unwrap();
    let registry = global_staleness_registry();
    registry.clear();
    let event = CdcOrderEvent {
        order_id: "order-123".to_string(),
        change: OrderChange::Created,
        commit_ts: SystemTime::now() - Duration::from_secs(5),
    };
    handle_cdc_orders(event, |_| Ok(()))?;
    let samples = registry.snapshot();
    let cdc = find_sample(&samples, "cdc_topic:orders").expect("cdc sample");
    assert_eq!(cdc.domain, "cdc");
    assert!(cdc.freshness_seconds >= 0.0);
    Ok(())
}

#[test]
fn chain_header_uses_timestamp_when_available() -> Result<(), HandlerError> {
    let _guard = TEST_GUARD.lock().unwrap();
    let registry = global_staleness_registry();
    registry.clear();
    let event = ChainHeader {
        height: 1024,
        hash: [1; 32],
        parent_hash: [2; 32],
        timestamp: Some(SystemTime::now() - Duration::from_secs(2)),
    };
    handle_chain_header(event, |_| Ok(()))?;
    let samples = registry.snapshot();
    let chain = find_sample(&samples, "chain_header").expect("chain sample");
    assert_eq!(chain.domain, "chain");
    assert!(chain.freshness_seconds >= 0.0);
    Ok(())
}

#[test]
fn invalid_events_return_errors() {
    let _guard = TEST_GUARD.lock().unwrap();
    let bad_oracle = OracleEvent {
        instrument: "".to_string(),
        best_bid: 1.0,
        best_ask: 1.0,
        origin_timestamp: None,
    };
    assert!(matches!(
        handle_oracle_event(bad_oracle, |_| Ok(())),
        Err(HandlerError::Invalid(_))
    ));
    let bad_market = MarketFeedEvent {
        venue: "".to_string(),
        symbol: "A".to_string(),
        last_price: -1.0,
        size: 1.0,
        sequence: 0,
        exchange_timestamp: None,
    };
    assert!(matches!(
        handle_market_feed(bad_market, |_| Ok(())),
        Err(HandlerError::Invalid(_))
    ));
    let bad_chain = ChainHeader {
        height: 0,
        hash: [0; 32],
        parent_hash: [0; 32],
        timestamp: None,
    };
    assert!(matches!(
        handle_chain_header(bad_chain, |_| Ok(())),
        Err(HandlerError::Invalid(_))
    ));
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
