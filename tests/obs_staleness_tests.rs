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
}
