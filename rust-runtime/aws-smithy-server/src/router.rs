// use std::{convert::Infallible};
use async_trait::async_trait;
use axum::{
    body::{box_body, BoxBody},
    response::IntoResponse,
};
use std::pin::Pin;

// use axum::body::BoxBody;
use futures::Future;
use http::{Request, Response};
use hyper::Body;
// use tower::Service;

// The `Handler` trait in axum is sealed, so we can't implement it outside axum nor specify it as
// the return type of our router; so here is our own take on that trait.
pub trait Handler1: Send + Sync + 'static {
    fn call<'a>(&'a self, req: Request<Body>) -> Pin<Box<dyn Future<Output = Response<Body>> + Send + 'a>>;
}

impl<F, R> Handler1 for F
where
    F: Send + Sync + 'static + Fn(Request<Body>) -> R,
    R: Future<Output = Response<Body>> + Send + 'static,
{
    fn call<'a>(&'a self, req: Request<Body>) -> Pin<Box<dyn Future<Output = Response<Body>> + Send + 'a>> {
        let fut = (self)(req);
        Box::pin(async move { fut.await })
    }
}

#[async_trait]
pub trait Handler<B>: Send + Sync + 'static {
    /// Call the handler with the given request.
    async fn call(&self, req: Request<B>) -> Response<BoxBody>;
}

#[async_trait]
impl<F, Fut, B, Res> Handler<B> for F
where
    // TODO Ideally I want `FnOnce` here but that would require taking ownership of `self`. axum
    // does it by cloning.
    F: Fn() -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Res> + Send,
    Res: IntoResponse,
    B: Send + 'static,
{
    async fn call(&self, _req: Request<B>) -> Response<BoxBody> {
        self().await.into_response().map(box_body)
    }
}

pub struct Router<T> {
    pub routes: Vec<T>,
}

impl<T> Router<T> {
    // TODO
    pub fn route() -> Self {
        unimplemented!()
    }

    // pub fn find<B>(
    //     self: &Self,
    //     request: &Request<B>,
    // ) -> Box<
    //     dyn Service<
    //         Request<B>,
    //         Response = Response<BoxBody>,
    //         Error = Infallible,
    //         Future = Pin<Box<dyn Future<Output = Result<Response<BoxBody>, Infallible>> + Send + 'static>>,
    //     >,
    // > {
    //     unimplemented!()
    // }

    pub fn find<B>(self: &Self, _request: &Request<B>) -> T {
        unimplemented!()
    }
}
