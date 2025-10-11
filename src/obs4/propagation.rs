use std::collections::BTreeMap;

use opentelemetry::{
    global,
    propagation::{Extractor, Injector},
    trace::{Link, SpanContext},
    Context,
};
use opentelemetry_http::{HeaderExtractor, HeaderInjector};

/// Inject the current [`Context`] into an [`http::request::Builder`].
///
/// This helper wraps [`HeaderInjector`] and assumes the global text map propagator
/// is configured with a W3C Trace Context implementation (for example
/// [`opentelemetry_sdk::propagation::TraceContextPropagator`]).
pub fn inject_headers(req: &mut http::request::Builder) {
    if let Some(headers) = req.headers_mut() {
        global::get_text_map_propagator(|propagator| {
            propagator.inject_context(&Context::current(), &mut HeaderInjector(headers));
        });
    }
}

/// Extract a [`Context`] from incoming HTTP headers.
///
/// The returned [`Context`] can be used as the parent when constructing new spans:
///
/// ```ignore
/// let parent_cx = extract_parent(request.headers());
/// let tracer = opentelemetry::global::tracer("obs4-http");
/// let mut span = tracer.start_with_context("http.server", &parent_cx);
/// span.end();
/// ```
pub fn extract_parent(headers: &http::HeaderMap) -> Context {
    global::get_text_map_propagator(|propagator| propagator.extract(&HeaderExtractor(headers)))
}

#[cfg(feature = "grpc-tonic")]
use tonic::metadata::{KeyAndValueRef, MetadataMap, MetadataValue};

#[cfg(feature = "grpc-tonic")]
struct MetadataMapInjector<'a> {
    metadata: &'a mut MetadataMap,
}

#[cfg(feature = "grpc-tonic")]
impl<'a> MetadataMapInjector<'a> {
    fn new(metadata: &'a mut MetadataMap) -> Self {
        Self { metadata }
    }
}

#[cfg(feature = "grpc-tonic")]
impl<'a> Injector for MetadataMapInjector<'a> {
    fn set(&mut self, key: &str, value: String) {
        if let Ok(parsed) = MetadataValue::try_from(value.as_str()) {
            let _ = self.metadata.insert(key, parsed);
        }
    }
}

#[cfg(feature = "grpc-tonic")]
struct MetadataMapExtractor<'a> {
    metadata: &'a MetadataMap,
}

#[cfg(feature = "grpc-tonic")]
impl<'a> MetadataMapExtractor<'a> {
    fn new(metadata: &'a MetadataMap) -> Self {
        Self { metadata }
    }
}

#[cfg(feature = "grpc-tonic")]
impl<'a> Extractor for MetadataMapExtractor<'a> {
    fn get(&self, key: &str) -> Option<&str> {
        self.metadata.get(key).and_then(|val| val.to_str().ok())
    }

    fn keys(&self) -> Vec<&str> {
        self.metadata
            .iter()
            .filter_map(|entry| match entry {
                KeyAndValueRef::Ascii(key, _) => Some(key.as_str()),
                KeyAndValueRef::Binary(_, _) => None,
            })
            .collect()
    }
}

#[cfg(feature = "grpc-tonic")]
use tonic::service::Interceptor;

#[cfg(feature = "grpc-tonic")]
use tonic::Request;

#[cfg(feature = "grpc-tonic")]
/// gRPC client interceptor that injects the current [`Context`] into request metadata.
pub fn client_interceptor() -> Interceptor {
    Interceptor::new(|mut req: Request<()>| {
        global::get_text_map_propagator(|propagator| {
            let mut injector = MetadataMapInjector::new(req.metadata_mut());
            propagator.inject_context(&Context::current(), &mut injector);
        });
        Ok(req)
    })
}

#[cfg(feature = "grpc-tonic")]
/// gRPC server interceptor that extracts the parent [`Context`] from request metadata.
///
/// The extracted context is stored inside the request extensions so handlers can
/// retrieve it and use it as the parent span when creating new traces.
pub fn server_interceptor() -> Interceptor {
    Interceptor::new(|mut req: Request<()>| {
        let parent = global::get_text_map_propagator(|propagator| {
            propagator.extract(&MetadataMapExtractor::new(req.metadata()))
        });
        req.extensions_mut().insert(parent);
        Ok(req)
    })
}

struct MapInjector<'a> {
    headers: &'a mut BTreeMap<String, String>,
}

impl<'a> MapInjector<'a> {
    fn new(headers: &'a mut BTreeMap<String, String>) -> Self {
        Self { headers }
    }
}

impl<'a> Injector for MapInjector<'a> {
    fn set(&mut self, key: &str, value: String) {
        self.headers.insert(key.to_string(), value);
    }
}

struct MapExtractor<'a> {
    headers: &'a BTreeMap<String, String>,
}

impl<'a> MapExtractor<'a> {
    fn new(headers: &'a BTreeMap<String, String>) -> Self {
        Self { headers }
    }
}

impl<'a> Extractor for MapExtractor<'a> {
    fn get(&self, key: &str) -> Option<&str> {
        self.headers.get(key).map(|value| value.as_str())
    }

    fn keys(&self) -> Vec<&str> {
        self.headers.keys().map(|key| key.as_str()).collect()
    }
}

/// Inject the current context into a [`BTreeMap`]-backed header map.
pub fn inject_map(headers: &mut BTreeMap<String, String>) {
    global::get_text_map_propagator(|propagator| {
        let mut injector = MapInjector::new(headers);
        propagator.inject_context(&Context::current(), &mut injector);
    });
}

/// Extract a [`Context`] from a [`BTreeMap`]-backed header map.
pub fn extract_map(headers: &BTreeMap<String, String>) -> Context {
    global::get_text_map_propagator(|propagator| propagator.extract(&MapExtractor::new(headers)))
}

/// Build a [`Link`] from a CDC span context to an `amm.swap` span.
pub fn link_from_cdc(cdc_ctx: &SpanContext) -> Link {
    Link::with_context(cdc_ctx.clone())
}
