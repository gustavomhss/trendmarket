use std::io::{Read, Write};
use std::net::{Shutdown, TcpListener, TcpStream};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

fn bind_free_port() -> (TcpListener, String) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind ephemeral port");
    let addr = listener.local_addr().expect("local addr");
    (listener, format!("{}", addr))
}

fn read_metrics(mut stream: TcpStream) -> std::io::Result<String> {
    stream.set_read_timeout(Some(Duration::from_secs(2)))?;
    let request = b"GET /metrics HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n";
    stream.write_all(request)?;
    stream.shutdown(Shutdown::Write)?;
    let mut raw = String::new();
    stream.read_to_string(&mut raw)?;
    Ok(raw.split("\r\n\r\n").nth(1).unwrap_or_default().to_string())
}

#[test]
fn prom_scrape_enabled_exposes_metrics() {
    let (listener, addr) = bind_free_port();
    drop(listener); // free the port for the child process

    let binary = env!("CARGO_BIN_EXE_obs_demo");
    let child = Command::new(binary)
        .env("DEPLOY_ENV", "dev")
        .env("OBSERVABILITY_LEVEL", "full")
        .env("PROM_SCRAPE", "on")
        .env("METRICS_HTTP_ADDR", &addr)
        .env("OBS_DEMO_OPS", "5")
        .stdout(Stdio::piped())
        .spawn()
        .expect("spawn obs_demo");

    let start = Instant::now();
    let metrics_body = loop {
        if start.elapsed() > Duration::from_secs(5) {
            panic!("timed out waiting for metrics endpoint");
        }
        match TcpStream::connect(&addr) {
            Ok(stream) => match read_metrics(stream) {
                Ok(body)
                    if body.contains("amm_op_latency_seconds_bucket")
                        && body.contains("hook_executions_total") =>
                {
                    break body
                }
                Ok(_) => std::thread::sleep(Duration::from_millis(50)),
                Err(err) => {
                    eprintln!("metrics request failed: {}", err);
                    std::thread::sleep(Duration::from_millis(50));
                }
            },
            Err(_) => std::thread::sleep(Duration::from_millis(50)),
        }
    };

    let output = child.wait_with_output().expect("wait for obs_demo");
    assert!(
        output.status.success(),
        "obs_demo exited with error: {:?}",
        output.status
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Prometheus exporter listening"));
    assert!(stdout.contains("obs_demo completed"));

    assert!(metrics_body.contains("amm_op_latency_seconds_bucket"));
    assert!(metrics_body.contains("hook_executions_total"));
}

#[test]
fn otlp_disabled_exits_cleanly() {
    let binary = env!("CARGO_BIN_EXE_obs_demo");
    let output = Command::new(binary)
        .env("DEPLOY_ENV", "dev")
        .env("OBSERVABILITY_LEVEL", "off")
        .env("PROM_SCRAPE", "off")
        .env("OBS_DEMO_OPS", "3")
        .stdout(Stdio::piped())
        .spawn()
        .expect("spawn obs_demo")
        .wait_with_output()
        .expect("wait for obs_demo");

    assert!(
        output.status.success(),
        "obs_demo exited with error: {:?}",
        output.status
    );
}
