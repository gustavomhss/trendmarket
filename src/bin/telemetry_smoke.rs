#[cfg(feature = "obs")]
use std::env;
#[cfg(feature = "obs")]
use std::io::{Read, Write};
#[cfg(feature = "obs")]
use std::net::{SocketAddr, TcpListener};
#[cfg(feature = "obs")]
use std::process::Command;
#[cfg(feature = "obs")]
use std::thread;
#[cfg(feature = "obs")]
use std::time::{Duration, Instant};

#[cfg(feature = "obs")]
use credit_engine_core::telemetry;

#[cfg(feature = "obs")]
fn main() {
    if std::env::args().any(|arg| arg == "--serve-metrics") {
        serve_metrics();
        return;
    }

    let addr = env::var("AMM_METRICS_ADDR").unwrap_or_else(|_| "127.0.0.1:9464".to_string());
    let prom_handles = telemetry::start_prometheus(&addr).expect("start prometheus");

    telemetry::inc_swap("CE-PAIR-TEST");
    telemetry::inc_liquidity("mint");
    telemetry::inc_error("CE-AMM-0000");
    telemetry::observe_swap_latency_ms(2.4);

    let snapshot = prom_handles.render();
    spawn_server(&addr, &snapshot);

    thread::sleep(Duration::from_millis(800));
}

#[cfg(not(feature = "obs"))]
fn main() {}

#[cfg(feature = "obs")]
fn spawn_server(addr: &str, payload: &str) {
    if let Ok(current_exe) = env::current_exe() {
        let mut cmd = Command::new(current_exe);
        cmd.arg("--serve-metrics")
            .env("AMM_METRICS_ADDR", addr)
            .env("TELEMETRY_METRICS_PAYLOAD", payload)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null());
        match cmd.spawn() {
            Ok(_) => {
                thread::sleep(Duration::from_millis(200));
            }
            Err(err) => {
                eprintln!("telemetry_smoke: failed to spawn metrics server: {}", err);
            }
        }
    }
}

#[cfg(feature = "obs")]
fn serve_metrics() {
    let addr = env::var("AMM_METRICS_ADDR").unwrap_or_else(|_| "127.0.0.1:9464".to_string());
    let payload = env::var("TELEMETRY_METRICS_PAYLOAD").unwrap_or_default();
    let socket: SocketAddr = addr.parse().expect("valid socket address");
    let listener = TcpListener::bind(socket).expect("bind metrics listener");
    listener.set_nonblocking(true).expect("set nonblocking");
    eprintln!("telemetry_smoke: metrics server listening on {}", addr);
    let deadline = Instant::now() + Duration::from_secs(120);
    loop {
        match listener.accept() {
            Ok((mut stream, _)) => {
                let mut buffer = [0_u8; 1024];
                let _ = stream.read(&mut buffer);
                let response = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: text/plain; version=0.0.4\r\nContent-Length: {}\r\nCache-Control: no-store\r\n\r\n{}",
                    payload.len(),
                    payload
                );
                let _ = stream.write_all(response.as_bytes());
                let _ = stream.flush();
                eprintln!("telemetry_smoke: served metrics response");
                break;
            }
            Err(err) => {
                if err.kind() == std::io::ErrorKind::WouldBlock {
                    if Instant::now() >= deadline {
                        eprintln!("telemetry_smoke: server timeout waiting for scrape");
                        break;
                    }
                    thread::sleep(Duration::from_millis(50));
                } else {
                    eprintln!("telemetry_smoke: server error: {}", err);
                    break;
                }
            }
        }
    }
}
