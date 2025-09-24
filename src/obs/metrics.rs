#![cfg(feature = "obs")]
use prometheus::Encoder; // traz o trait p/ TextEncoder::encode

pub fn start_metrics_http(addr: &str) {
    let addr = addr.to_string();
    std::thread::spawn(move || {
        let server = tiny_http::Server::http(&addr).expect("metrics server bind");
        eprintln!("[obs] /metrics at http://{}/metrics", addr);

        for req in server.incoming_requests() {
            if req.url() == "/metrics" {
                let body = prometheus_text();
                let hdr = tiny_http::Header::from_bytes(
                    &b"Content-Type"[..],
                    &b"text/plain; version=0.0.4"[..],
                )
                .unwrap();
                let resp = tiny_http::Response::from_string(body).with_header(hdr);
                let _ = req.respond(resp);
            } else {
                let _ = req.respond(
                    tiny_http::Response::from_string("not found").with_status_code(404),
                );
            }
        }
    });
}

fn prometheus_text() -> String {
    let enc = prometheus::TextEncoder::new();
    let mut buf = Vec::new();
    let reg = crate::obs::init::PROM_REGISTRY
        .get()
        .expect("prom registry")
        .clone();
    let mf = reg.gather();
    let _ = enc.encode(&mf, &mut buf);
    String::from_utf8(buf).expect("utf8 metrics")
}
