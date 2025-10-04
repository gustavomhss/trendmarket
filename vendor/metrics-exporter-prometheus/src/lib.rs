use metrics::render_prometheus;
use once_cell::sync::OnceCell;
use std::net::SocketAddr;

pub struct PrometheusBuilder {
    addr: Option<SocketAddr>,
}

#[derive(Clone, Copy)]
pub struct PrometheusHandle {
    addr: SocketAddr,
}

static LAST_ADDR: OnceCell<SocketAddr> = OnceCell::new();

impl PrometheusBuilder {
    pub fn new() -> Self {
        Self { addr: None }
    }

    pub fn with_http_listener(mut self, addr: SocketAddr) -> Self {
        self.addr = Some(addr);
        self
    }

    pub fn install(self) -> std::io::Result<PrometheusHandle> {
        if let Some(addr) = self.addr {
            let _ = LAST_ADDR.set(addr);
            Ok(PrometheusHandle { addr })
        } else {
            Err(std::io::Error::new(
                std::io::ErrorKind::AddrNotAvailable,
                "listener address not set",
            ))
        }
    }
}

pub fn configured_addr() -> Option<SocketAddr> {
    LAST_ADDR.get().copied()
}

impl PrometheusHandle {
    pub fn render(&self) -> String {
        render_prometheus()
    }

    pub fn addr(&self) -> SocketAddr {
        self.addr
    }
}
