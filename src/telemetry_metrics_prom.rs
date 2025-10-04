use std::convert::Infallible;
use std::net::SocketAddr;
use std::sync::Arc;

use bytes::Bytes;
use http_body_util::{combinators::BoxBody, BodyExt, Full};
use hyper::http::{header::CONTENT_TYPE, Method, Request, Response, StatusCode};
use hyper::{body::Incoming, service::service_fn};
use hyper_util::{
    rt::{TokioExecutor, TokioIo},
    server::conn::auto::Builder as HyperBuilder,
};
use prometheus::{Encoder, TextEncoder};
use tokio::net::TcpListener;
use tokio::runtime::Handle;
use tokio::sync::oneshot;
use tokio::task::JoinHandle;

use opentelemetry_sdk::metrics::SdkMeterProvider;
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PromServerConfig {
    pub addr: String,
}

pub struct PromExporter {
    pub registry: prometheus::Registry,
    provider: Arc<SdkMeterProvider>,
}

impl PromExporter {
    pub fn meter_provider(&self) -> Arc<SdkMeterProvider> {
        Arc::clone(&self.provider)
    }
}

pub struct PromServerGuard {
    shutdown: Option<oneshot::Sender<()>>,
    handle: Option<JoinHandle<Result<(), PromHttpError>>>,
    addr: SocketAddr,
    exporter: Arc<PromExporter>,
}

impl PromServerGuard {
    pub fn local_addr(&self) -> SocketAddr {
        self.addr
    }

    pub fn exporter(&self) -> Arc<PromExporter> {
        Arc::clone(&self.exporter)
    }

    pub async fn shutdown(mut self) {
        if let Some(tx) = self.shutdown.take() {
            let _ = tx.send(());
        }
        if let Some(handle) = self.handle.take() {
            let _ = handle.await;
        }
    }
}

impl Drop for PromServerGuard {
    fn drop(&mut self) {
        if let Some(tx) = self.shutdown.take() {
            let _ = tx.send(());
        }
        if let Some(handle) = self.handle.take() {
            if let Ok(rt) = Handle::try_current() {
                rt.spawn(async move {
                    let _ = handle.await;
                });
            } else {
                handle.abort();
            }
        }
    }
}

pub fn init_prom_exporter() -> PromExporter {
    let registry = prometheus::Registry::new();
    let provider = Arc::new(
        SdkMeterProvider::builder()
            .with_reader(
                opentelemetry_prometheus::exporter()
                    .with_registry(registry.clone())
                    .build()
                    .expect("failed to build Prometheus exporter"),
            )
            .build(),
    );

    PromExporter { registry, provider }
}

pub async fn spawn_metrics_http(
    cfg: PromServerConfig,
    exporter: PromExporter,
) -> Result<PromServerGuard, PromHttpError> {
    let addr: SocketAddr = cfg
        .addr
        .parse()
        .map_err(|err: std::net::AddrParseError| PromHttpError::Bind(err.to_string()))?;

    let listener = TcpListener::bind(addr)
        .await
        .map_err(|err| PromHttpError::Bind(err.to_string()))?;
    let local_addr = listener
        .local_addr()
        .map_err(|err| PromHttpError::Bind(err.to_string()))?;

    let exporter = Arc::new(exporter);
    let (shutdown_tx, shutdown_rx) = oneshot::channel();
    let server_exporter = exporter.clone();

    let handle =
        tokio::spawn(async move { run_server(listener, server_exporter, shutdown_rx).await });

    Ok(PromServerGuard {
        shutdown: Some(shutdown_tx),
        handle: Some(handle),
        addr: local_addr,
        exporter,
    })
}

async fn run_server(
    listener: TcpListener,
    exporter: Arc<PromExporter>,
    mut shutdown: oneshot::Receiver<()>,
) -> Result<(), PromHttpError> {
    let builder = HyperBuilder::new(TokioExecutor::new()).http1_only();

    loop {
        tokio::select! {
            _ = &mut shutdown => {
                break;
            }
            accept_result = listener.accept() => {
                let (stream, _) = accept_result.map_err(|err| PromHttpError::Serve(err.to_string()))?;
                let service_exporter = exporter.clone();
                let service = service_fn(move |req| {
                    let exporter = service_exporter.clone();
                    async move { handle_request(req, exporter).await }
                });
                let io = TokioIo::new(stream);
                if let Err(err) = builder.serve_connection(io, service).await {
                    return Err(PromHttpError::Serve(err.to_string()));
                }
            }
        }
    }

    Ok(())
}

async fn handle_request(
    request: Request<Incoming>,
    exporter: Arc<PromExporter>,
) -> Result<Response<BoxBody<Bytes, Infallible>>, Infallible> {
    if request.method() != Method::GET || request.uri().path() != "/metrics" {
        let response = Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Full::new(Bytes::new()).boxed())
            .expect("failed to build 404 response");
        return Ok(response);
    }

    match gather_and_encode(&exporter) {
        Ok(body) => {
            let response = Response::builder()
                .status(StatusCode::OK)
                .header(CONTENT_TYPE, "text/plain; version=0.0.4; charset=utf-8")
                .body(Full::new(Bytes::from(body)).boxed())
                .expect("failed to build metrics response");
            Ok(response)
        }
        Err(err) => {
            let response = Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .header(CONTENT_TYPE, "text/plain; version=0.0.4; charset=utf-8")
                .body(Full::new(Bytes::from(err)).boxed())
                .expect("failed to build error response");
            Ok(response)
        }
    }
}

fn gather_and_encode(exporter: &PromExporter) -> Result<Vec<u8>, String> {
    let metric_families = exporter.registry.gather();
    let mut buffer = Vec::new();
    let encoder = TextEncoder::new();
    encoder
        .encode(&metric_families, &mut buffer)
        .map_err(|err| err.to_string())?;
    Ok(buffer)
}

#[derive(Error, Debug)]
pub enum PromHttpError {
    #[error("failed to bind HTTP listener: {0}")]
    Bind(String),
    #[error("failed to serve metrics: {0}")]
    Serve(String),
}
