use std::time::{Duration, Instant};

use credit_engine_core::obs_staleness::StalenessClock;

fn assert_close(actual: f64, expected: f64, tolerance: f64) {
    assert!(
        (actual - expected).abs() <= tolerance,
        "expected {expected}±{tolerance}, got {actual}"
    );
}

#[test]
fn clamp_zero_on_clock_step() {
    let base = Instant::now();
    let mut clock = StalenessClock::with_initial_arrival(base);
    let earlier = base - Duration::from_millis(15);
    let snapshot = clock.on_clock_step(earlier);
    assert!(snapshot.staleness_seconds >= 0.0);
    assert_close(snapshot.staleness_seconds, 0.0, 1.0e-6);
    assert!(snapshot.freshness_seconds >= 0.0);
    assert_close(snapshot.freshness_seconds, 0.0, 1.0e-6);
    assert_eq!(snapshot.recency_seconds, Some(0.0));
}

#[test]
fn monotonic_between_arrivals() {
    let base = Instant::now();
    let mut clock = StalenessClock::new();
    clock.on_arrival(base);
    let first = clock.on_clock_step(base + Duration::from_millis(12));
    let second = clock.on_clock_step(base + Duration::from_millis(27));
    assert!(second.staleness_seconds + 1.0e-6 >= first.staleness_seconds);
    assert!(second.freshness_seconds + 1.0e-6 >= first.freshness_seconds);
    assert!(first.staleness_seconds >= 0.0);
    assert!(second.staleness_seconds >= first.staleness_seconds - 1.0e-6);
}

#[test]
fn reset_on_arrival() {
    let base = Instant::now();
    let mut clock = StalenessClock::new();
    clock.on_arrival(base);
    let _ = clock.on_clock_step(base + Duration::from_millis(30));
    assert!(clock.staleness_seconds() > 0.0);
    let reset_snapshot = clock.on_arrival(base + Duration::from_millis(45));
    assert_close(reset_snapshot.staleness_seconds, 0.0, 1.0e-6);
    assert_eq!(reset_snapshot.recency_seconds, Some(0.0));
    let after_reset = clock.on_clock_step(base + Duration::from_millis(60));
    assert!(after_reset.staleness_seconds >= 0.0);
    assert_close(after_reset.staleness_seconds, 0.015, 0.1);
}

#[test]
fn optional_recency() {
    let mut clock = StalenessClock::new();
    let pre = clock.on_clock_step(Instant::now());
    assert!(pre.recency_seconds.is_none());
    let event_time = Instant::now() + Duration::from_millis(5);
    clock.on_arrival(event_time);
    let post = clock.on_clock_step(event_time + Duration::from_millis(10));
    assert!(post.recency_seconds.is_some());
    assert!(post.recency_seconds.unwrap() >= 0.0);
}
