use std::error::Error;
use std::sync::Arc;
use std::time::Duration;

use credit_engine_core::telemetry_metrics_prom::{
    init_prom_exporter, spawn_metrics_http, PromServerConfig,
};
use opentelemetry::metrics::MeterProvider as _;
use tokio::{io::AsyncReadExt, io::AsyncWriteExt};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn prometheus_http_server_exposes_metrics() -> Result<(), Box<dyn Error>> {
    let exporter = init_prom_exporter();
    let provider = exporter.meter_provider();
    let meter = provider.meter("test-prom");
    let counter = meter.u64_counter("test_counter_total").build();
    counter.add(3, &[]);

    let histogram = meter.f64_histogram("test_histogram_seconds").build();
    histogram.record(0.25, &[]);
    histogram.record(1.0, &[]);
    histogram.record(2.75, &[]);

    let guard = spawn_metrics_http(
        PromServerConfig {
            addr: "127.0.0.1:0".to_string(),
        },
        exporter,
    )
    .await?;

    tokio::time::sleep(Duration::from_millis(50)).await;

    let guard_exporter = guard.exporter();
    let guard_provider = guard_exporter.meter_provider();
    assert!(Arc::ptr_eq(&provider, &guard_provider));
    let guard_meter = guard_provider.meter("test-prom-guard");
    let late_counter = guard_meter.u64_counter("late_counter_total").build();
    late_counter.add(1, &[]);

    let addr = guard.local_addr();
    println!("bound on {}", addr);
    let mut stream = tokio::net::TcpStream::connect(addr).await?;
    let request = format!(
        "GET /metrics HTTP/1.1\r\nHost: {}\r\nConnection: close\r\n\r\n",
        addr
    );
    stream.write_all(request.as_bytes()).await?;

    let mut buffer = Vec::new();
    stream.read_to_end(&mut buffer).await?;
    let response = String::from_utf8_lossy(&buffer);

    assert!(
        response.starts_with("HTTP/1.1 200"),
        "unexpected status: {response}"
    );
    assert!(
        response
            .to_ascii_lowercase()
            .contains("content-type: text/plain; version=0.0.4; charset=utf-8"),
        "missing content-type header: {response}"
    );

    let body = response.split("\r\n\r\n").nth(1).unwrap_or("");
    let preview: String = body.lines().take(6).collect::<Vec<_>>().join("\n");
    println!("metrics preview:\n{}", preview);
    assert!(
        body.contains("test_counter_total"),
        "counter not present: {body}"
    );
    assert!(body.contains("test_histogram_seconds_bucket"));
    assert!(body.contains("test_histogram_seconds_sum"));
    assert!(body.contains("test_histogram_seconds_count"));
    assert!(body.contains("le=\"+Inf\""));
    assert!(body.contains("late_counter_total"));

    guard.shutdown().await;

    Ok(())
}
