use crate::body::{box_body, BoxBody};
use axum::handler::Handler;
use http::{Request, Response};
use std::{
    convert::Infallible,
    future::Future,
    marker::PhantomData,
    pin::Pin,
    task::{Context, Poll},
};
use tower_service::Service;

pub struct MethodRouter<H, B, T> {
    pub(crate) handler: H,
    pub(crate) _marker: PhantomData<fn() -> (B, T)>,
}

impl<H, B, T> Clone for MethodRouter<H, B, T>
where
    H: Clone,
{
    fn clone(&self) -> Self {
        Self { handler: self.handler.clone(), _marker: PhantomData }
    }
}

pub fn operation<H, B, T>(handler: H) -> MethodRouter<H, B, T> {
    MethodRouter { handler, _marker: PhantomData }
}

impl<H, B, T> Service<Request<B>> for MethodRouter<H, B, T>
where
    H: Handler<B, T>,
    B: Send + 'static,
{
    type Response = Response<BoxBody>;
    type Error = Infallible;
    // TODO Use `pin_project`.
    // TODO Is axum's future `Send`?
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    #[inline]
    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: Request<B>) -> Self::Future {
        let req_method = req.method().clone();
        let handler_clone = self.handler.clone();

        // Ugly code to convert `Response<axum::body::BoxBody>` to `Reponse<crate::body::BoxBody>`.
        let fut = async {
            let resp = Handler::call(handler_clone, req).await;
            let resp = resp.map(|b| box_body(b));
            Ok(resp)
        };
        Box::pin(fut)
    }
}
