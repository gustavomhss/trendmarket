use std::task::{Context, Poll};
use std::time::Duration;

use http::Uri;
use tower_service::Service;

#[derive(Debug)]
pub struct TimeoutConnector<C> {
    inner: C,
    connect_timeout: Option<Duration>,
}

impl<C> TimeoutConnector<C> {
    pub fn new(inner: C) -> Self {
        Self {
            inner,
            connect_timeout: None,
        }
    }

    pub fn set_connect_timeout(&mut self, timeout: Option<Duration>) {
        self.connect_timeout = timeout;
    }

    pub fn connect_timeout(&self) -> Option<Duration> {
        self.connect_timeout
    }
}

impl<C> Clone for TimeoutConnector<C>
where
    C: Clone,
{
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            connect_timeout: self.connect_timeout,
        }
    }
}

impl<C> Service<Uri> for TimeoutConnector<C>
where
    C: Service<Uri>,
{
    type Response = C::Response;
    type Error = C::Error;
    type Future = C::Future;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Uri) -> Self::Future {
        let _timeout = self.connect_timeout;
        self.inner.call(req)
    }
}
