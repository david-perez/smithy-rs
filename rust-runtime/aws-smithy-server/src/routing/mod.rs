use self::{future::RouterFuture, request_spec::RequestSpec};
use crate::body::{Body, BoxBody};
use http::{Request, Response};
use std::{
    collections::HashMap,
    convert::Infallible,
    task::{Context, Poll},
};
use tower::util::ServiceExt;
use tower_service::Service;

pub mod future;
mod into_make_service;
pub mod method_router;
mod not_found;
pub mod request_spec;
mod route;

pub use self::{into_make_service::IntoMakeService, route::Route};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct RouteId(u64);

impl RouteId {
    fn next() -> Self {
        use std::sync::atomic::{AtomicU64, Ordering};
        static ID: AtomicU64 = AtomicU64::new(0);
        Self(ID.fetch_add(1, Ordering::SeqCst))
    }
}

#[derive(Debug)]
pub struct Router<B = Body> {
    routes: HashMap<RouteId, Route<B>>,
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

        let id = RouteId::next();

        self.routes.insert(id, Route::new(svc, request_spec));

        self
    }

    /// Apply a [`tower::Layer`] to the router.
    // TODO

    /// Convert this router into a [`MakeService`], that is a [`Service`] whose
    /// response is another service.
    ///
    /// This is useful when running your application with hyper's
    /// [`Server`](hyper::server::Server):
    ///
    /// [`MakeService`]: tower::make::MakeService
    pub fn into_make_service(self) -> IntoMakeService<Self> {
        IntoMakeService::new(self)
    }
}

impl<B> Service<Request<B>> for Router<B>
where
    B: Send + Sync + 'static,
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
        // TODO Do we need this?
        // if req.extensions().get::<OriginalUri>().is_none() {
        //     let original_uri = OriginalUri(req.uri().clone());
        //     req.extensions_mut().insert(original_uri);
        // }

        for (_, route) in self.routes.iter() {
            match route.matches(&req) {
                request_spec::Match::Yes => {
                    return RouterFuture::from_oneshot(route.clone().oneshot(req));
                }
                request_spec::Match::No => todo!(),
                request_spec::Match::MethodNotAllowed => todo!(),
            }
        }

        // TODO Return `404 Not Found`.
        todo!();
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
