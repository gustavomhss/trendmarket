use std::{io::Write, thread};
use anyhow::Result;

pub fn spawn(addr: &str) -> Result<std::thread::JoinHandle<()>> {
    let addr = addr.to_string();
    let handle = thread::spawn(move || {
        let server = tiny_http::Server::http(&addr).expect("metrics server bind");
        for req in server.incoming_requests() {
            if req.url() == "/metrics" {
                let body = encode_prometheus();
                let mut resp = tiny_http::Response::from_data(body);
                let _ = req.respond(resp.with_status_code(200));
            } else {
                let _ = req.respond(tiny_http::Response::from_string("not found").with_status_code(404));
            }
        }
    });
    Ok(handle)
}

fn encode_prometheus() -> Vec<u8> {
    let mut buf = Vec::new();
    if let Some(exp) = crate::obs::init::EXPORTER.get() {
        let mf = exp.registry().gather();
        let enc = prometheus::TextEncoder::new();
        let _ = enc.encode(&mf, &mut buf);
    } else {
        let _ = writeln!(&mut buf, "# exporter not ready");
    }
    buf
}
