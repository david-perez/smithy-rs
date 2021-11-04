use self::{future::RouterFuture, request_spec::RequestSpec};
use crate::body::{Body, BoxBody};
use http::{Request, Response, StatusCode};
use std::{
    convert::Infallible,
    task::{Context, Poll},
};
use tower::util::ServiceExt;
use tower_service::Service;

pub mod future;
mod into_make_service;
pub mod operation_handler;
pub mod request_spec;
mod route;

pub use self::{into_make_service::IntoMakeService, route::Route};

#[derive(Debug)]
pub struct Router<B = Body> {
    routes: Vec<Route<B>>,
}

impl<B> Clone for Router<B> {
    fn clone(&self) -> Self {
        Self { routes: self.routes.clone() }
    }
}

impl<B> Default for Router<B>
where
    B: Send + 'static,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<B> Router<B>
where
    B: Send + 'static,
{
    /// Create a new `Router`.
    ///
    /// Unless you add additional routes this will respond to `404 Not Found` to
    /// all requests.
    pub fn new() -> Self {
        Self { routes: Default::default() }
    }

    /// Add a route to the router.
    pub fn route<T>(mut self, request_spec: RequestSpec, svc: T) -> Self
    where
        T: Service<Request<B>, Response = Response<BoxBody>, Error = Infallible> + Clone + Send + 'static,
        T::Future: Send + 'static,
    {
        let svc = match try_downcast::<Router<B>, _>(svc) {
            Ok(_) => {
                panic!("Invalid route: `Router::route` cannot be used with `Router`s.")
            }
            Err(svc) => svc,
        };

        self.routes.push(Route::new(svc, request_spec));
        self
    }

    /// Apply a [`tower::Layer`] to the router.
    // TODO

    /// Convert this router into a [`MakeService`], that is a [`Service`] whose
    /// response is another service.
    ///
    /// This is useful when running your application with hyper's
    /// [`Server`](hyper::server::Server).
    ///
    /// [`MakeService`]: tower::make::MakeService
    pub fn into_make_service(self) -> IntoMakeService<Self> {
        IntoMakeService::new(self)
    }
}

impl<B> Service<Request<B>> for Router<B>
where
    B: Send + 'static,
{
    type Response = Response<BoxBody>;
    type Error = Infallible;
    type Future = RouterFuture<B>;

    #[inline]
    fn poll_ready(&mut self, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    #[inline]
    fn call(&mut self, req: Request<B>) -> Self::Future {
        let mut method_not_allowed = false;

        for route in &self.routes {
            match route.matches(&req) {
                request_spec::Match::Yes => {
                    return RouterFuture::from_oneshot(route.clone().oneshot(req));
                }
                request_spec::Match::MethodNotAllowed => method_not_allowed = true,
                request_spec::Match::No => {
                    // continue
                }
            }
        }

        let status_code = if method_not_allowed { StatusCode::METHOD_NOT_ALLOWED } else { StatusCode::NOT_FOUND };
        RouterFuture::from_response(Response::builder().status(status_code).body(crate::body::empty()).unwrap())
    }
}

fn try_downcast<T, K>(k: K) -> Result<T, K>
where
    T: 'static,
    K: Send + 'static,
{
    use std::any::Any;

    let k = Box::new(k) as Box<dyn Any + Send + 'static>;
    match k.downcast() {
        Ok(t) => Ok(*t),
        Err(other) => Err(*other.downcast().unwrap()),
    }
}
