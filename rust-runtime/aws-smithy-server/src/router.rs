// use std::{convert::Infallible};
use async_trait::async_trait;
use axum::{
    body::{box_body, BoxBody},
    extract::{FromRequest, RequestParts},
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
pub trait Handler<B, I>: Send + Sync + 'static {
    /// Call the handler with the given request.
    async fn call(&self, req: Request<B>) -> Response<BoxBody>;
}

// For multiple arguments, `I` can be a tuple.
#[async_trait]
impl<F, Fut, B, Res, I> Handler<B, I> for F
where
    // TODO Ideally I want `FnOnce` here but that would require taking ownership of `self`. axum
    // does it by requiring `Clone` (which is automatically implemented by function pointers, so I
    // guess it's ok).
    F: Fn(I) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Res> + Send,
    Res: IntoResponse,
    B: Send + 'static,
    I: FromRequest<B> + Send,
{
    async fn call(&self, req: Request<B>) -> Response<BoxBody> {
        let mut req = RequestParts::new(req);

        let input = match I::from_request(&mut req).await {
            Ok(value) => value,
            Err(rejection) => return rejection.into_response().map(box_body),
        };

        self(input).await.into_response().map(box_body)
    }
}

#[derive(Debug, Clone)]
pub struct Route<H, R> {
    pub matches: bool,
    pub handler: H,
    pub next_route: R,
}

#[async_trait]
pub trait Router {
    fn route() -> Self;

    async fn route_and_call<B>(self: &Self, req: Request<B>) -> Response<BoxBody>
    where
        // R: Router + Send,
        // H: Handler<B, I> + Send,
        // I: FromRequest<B> + Send,
        B: Send + 'static;
}

// pub struct Router<T> {
//     pub routes: Vec<T>,
// }

// impl<T> Router<T> {
//     // TODO
//     pub fn route() -> Self {
//         unimplemented!()
//     }

//     pub fn find<B>(self: &Self, _request: &Request<B>) -> T {
//         unimplemented!()
//     }
// }

#[derive(Debug, Clone)]
pub struct EmptyRouter;

#[async_trait]
impl<H, R> Router for Route<H, R>
where
    R: Router + Send + Sync,
    H: Send + Sync,
{
    fn route() -> Self {
        todo!()
    }

    async fn route_and_call<B>(self: &Self, req: Request<B>) -> Response<BoxBody>
    where
        // R: Router + Send,
        // H: Handler<B, I> + Send,
        // I: FromRequest<B> + Send,
        B: Send + 'static,
    {
        todo!()
    }
}

#[async_trait]
impl Router for EmptyRouter {
    fn route() -> Self {
        todo!()
    }

    async fn route_and_call<B>(self: &Self, _req: Request<B>) -> Response<BoxBody>
    where
        B: Send + 'static,
    {
        todo!()
    }
}
