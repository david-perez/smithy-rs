use crate::{body::BoxBody, handler::Handler};
use http::{Request, Response};
use std::{
    convert::Infallible,
    future::Future,
    marker::PhantomData,
    pin::Pin,
    task::{Context, Poll},
};
use tower::Service;

/// Struct that holds a handler, that is, a function provided by the user that implements the
/// Smithy operation.
pub struct OperationHandler<H, B, T> {
    handler: H,
    _marker: PhantomData<fn() -> (B, T)>,
}

impl<H, B, T> Clone for OperationHandler<H, B, T>
where
    H: Clone,
{
    fn clone(&self) -> Self {
        Self {
            handler: self.handler.clone(),
            _marker: PhantomData,
        }
    }
}

/// Construct an [`OperationHandler`] out of a function implementing the operation.
pub fn operation<H, B, T>(handler: H) -> OperationHandler<H, B, T> {
    OperationHandler {
        handler,
        _marker: PhantomData,
    }
}

impl<H, B, T> Service<Request<B>> for OperationHandler<H, B, T>
where
    H: Handler<B, T>,
    B: Send + 'static,
{
    type Response = Response<BoxBody>;
    type Error = Infallible;
    // TODO Implement our own future to avoid the pinned box.
    // Or at least use `opaque_future!` and `pin_project`.
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    #[inline]
    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: Request<B>) -> Self::Future {
        let handler_clone = self.handler.clone();

        let fut = async {
            let resp = Handler::call(handler_clone, req).await;
            Ok(resp)
        };
        Box::pin(fut)
    }
}
