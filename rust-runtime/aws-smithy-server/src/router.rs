// use std::{convert::Infallible};
use std::pin::Pin;

// use axum::body::BoxBody;
use futures::Future;
use http::{Request, Response};
use hyper::Body;
// use tower::Service;

// The `Handler` trait in axum is sealed, so we can't implement it outside axum nor specify it as
// the return type of our router; so here is our own take on that trait.
pub trait Handler: Send + Sync + 'static {
    fn call<'a>(&'a self, req: Request<Body>) -> Pin<Box<dyn Future<Output = Response<Body>> + Send + 'a>>;
}

impl<F, R> Handler for F
where
    F: Send + Sync + 'static + Fn(Request<Body>) -> R,
    R: Future<Output = Response<Body>> + Send + 'static,
{
    fn call<'a>(&'a self, req: Request<Body>) -> Pin<Box<dyn Future<Output = Response<Body>> + Send + 'a>> {
        let fut = (self)(req);
        Box::pin(async move { fut.await })
    }
}

pub struct Router<T> {
    pub routes: Vec<T>,
}

impl<T> Router<T> {
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
