use once_cell::sync::OnceCell;
use std::net::SocketAddr;

pub struct PrometheusBuilder {
    addr: Option<SocketAddr>,
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

    pub fn install(self) -> std::io::Result<()> {
        if let Some(addr) = self.addr {
            let _ = LAST_ADDR.set(addr);
            Ok(())
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
