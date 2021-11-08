use crate::{
    body::{Body, BoxBody},
    clone_box_service::CloneBoxService,
};
use http::{Request, Response};
use pin_project_lite::pin_project;
use std::{
    convert::Infallible,
    fmt,
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};
use tower::{util::Oneshot, ServiceExt};
use tower_service::Service;

use super::request_spec::{Match, RequestSpec};

/// How routes are stored inside a [`Router`](super::Router).
pub struct Route<B = Body> {
    service: CloneBoxService<Request<B>, Response<BoxBody>, Infallible>,
    request_spec: RequestSpec,
}

impl<B> Route<B> {
    pub(super) fn new<T>(svc: T, request_spec: RequestSpec) -> Self
    where
        T: Service<Request<B>, Response = Response<BoxBody>, Error = Infallible>
            + Clone
            + Send
            + 'static,
        T::Future: Send + 'static,
    {
        Self {
            service: CloneBoxService::new(svc),
            request_spec,
        }
    }

    pub(super) fn matches(&self, req: &Request<B>) -> Match {
        self.request_spec.matches(req)
    }
}

impl<ReqBody> Clone for Route<ReqBody> {
    fn clone(&self) -> Self {
        Self {
            service: self.service.clone(),
            request_spec: self.request_spec.clone(),
        }
    }
}

impl<ReqBody> fmt::Debug for Route<ReqBody> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Route").finish()
    }
}

impl<B> Service<Request<B>> for Route<B> {
    type Response = Response<BoxBody>;
    type Error = Infallible;
    type Future = RouteFuture<B>;

    #[inline]
    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    #[inline]
    fn call(&mut self, req: Request<B>) -> Self::Future {
        RouteFuture::new(self.service.clone().oneshot(req))
    }
}

pin_project! {
    /// Response future for [`Route`].
    pub struct RouteFuture<B> {
        #[pin]
        future: Oneshot<
            CloneBoxService<Request<B>, Response<BoxBody>, Infallible>,
            Request<B>,
        >
    }
}

impl<B> RouteFuture<B> {
    pub(crate) fn new(
        future: Oneshot<CloneBoxService<Request<B>, Response<BoxBody>, Infallible>, Request<B>>,
    ) -> Self {
        RouteFuture { future }
    }
}

impl<B> Future for RouteFuture<B> {
    type Output = Result<Response<BoxBody>, Infallible>;

    #[inline]
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        self.project().future.poll(cx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn traits() {
        use crate::test_helpers::*;

        assert_send::<Route<()>>();
    }
}
