use self::not_found::NotFound;
use self::{future::RouterFuture, request_spec::RequestSpec};
use crate::{
    body::{box_body, Body, BoxBody},
    util::{ByteStr, PercentDecodedByteStr},
};
use bytes::Bytes;
use http::{Request, Response, StatusCode, Uri};
use std::{
    borrow::Cow,
    collections::HashMap,
    convert::Infallible,
    fmt,
    sync::Arc,
    task::{Context, Poll},
};
use tower::{util::ServiceExt, ServiceBuilder};
use tower_http::map_response_body::MapResponseBodyLayer;
use tower_layer::Layer;
use tower_service::Service;

mod not_found;
mod route;
pub mod future;
pub mod request_spec;
pub mod method_router;
mod into_make_service;
mod method_not_allowed;

pub use self::{into_make_service::IntoMakeService, route::Route};
// TODO I think this should be public.
pub(crate) use self::method_not_allowed::MethodNotAllowed;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct RouteId(u64);

impl RouteId {
    fn next() -> Self {
        use std::sync::atomic::{AtomicU64, Ordering};
        static ID: AtomicU64 = AtomicU64::new(0);
        Self(ID.fetch_add(1, Ordering::SeqCst))
    }
}

pub struct Router<B = Body> {
    routes: HashMap<RouteId, Route<B>>,
    fallback: Fallback<B>,
}

impl<B> Clone for Router<B> {
    fn clone(&self) -> Self {
        Self {
            routes: self.routes.clone(),
            fallback: self.fallback.clone(),
        }
    }
}

impl<B> Default for Router<B>
where
    B: Send + Sync + 'static,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<B> fmt::Debug for Router<B> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Router")
            .field("routes", &self.routes)
            .field("fallback", &self.fallback)
            .finish()
    }
}

impl<B> Router<B>
where
    B: Send + Sync + 'static,
{
    /// Create a new `Router`.
    ///
    /// Unless you add additional routes this will respond to `404 Not Found` to
    /// all requests.
    pub fn new() -> Self {
        Self {
            routes: Default::default(),
            fallback: Fallback::Default(Route::new(NotFound, RequestSpec::always_get())),
        }
    }

    /// Add a route to the router.
    pub fn route<T>(mut self, request_spec: RequestSpec, svc: T) -> Self
    where
        T: Service<Request<B>, Response = Response<BoxBody>, Error = Infallible>
            + Clone
            + Send
            + 'static,
        T::Future: Send + 'static,
    {
        // TODO
        let svc = match try_downcast::<Router<B>, _>(svc) {
            Ok(_) => {
                panic!("Invalid route: `Router::route` cannot be used with `Router`s. Use `Router::nest` instead")
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
    fn call(&mut self, mut req: Request<B>) -> Self::Future {
        // TODO Do we need this?
        // if req.extensions().get::<OriginalUri>().is_none() {
        //     let original_uri = OriginalUri(req.uri().clone());
        //     req.extensions_mut().insert(original_uri);
        // }

        let path = req.uri().path().to_string();

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

// TODO Remove
// We store the potential error here such that users can handle invalid path
// params using `Result<Path<T>, _>`. That wouldn't be possible if we
// returned an error immediately when decoding the param
pub(crate) struct UrlParams(
    pub(crate) Result<Vec<(ByteStr, PercentDecodedByteStr)>, InvalidUtf8InPathParam>,
);

fn insert_url_params<B>(req: &mut Request<B>, params: Vec<(String, String)>) {
    let params = params
        .into_iter()
        .map(|(k, v)| {
            if let Some(decoded) = PercentDecodedByteStr::new(v) {
                Ok((ByteStr::new(k), decoded))
            } else {
                Err(InvalidUtf8InPathParam {
                    key: ByteStr::new(k),
                })
            }
        })
        .collect::<Result<Vec<_>, _>>();

    if let Some(current) = req.extensions_mut().get_mut::<Option<UrlParams>>() {
        match params {
            Ok(params) => {
                let mut current = current.take().unwrap();
                if let Ok(current) = &mut current.0 {
                    current.extend(params);
                }
                req.extensions_mut().insert(Some(current));
            }
            Err(err) => {
                req.extensions_mut().insert(Some(UrlParams(Err(err))));
            }
        }
    } else {
        req.extensions_mut().insert(Some(UrlParams(params)));
    }
}

pub(crate) struct InvalidUtf8InPathParam {
    pub(crate) key: ByteStr,
}

enum Fallback<B> {
    Default(Route<B>),
    Custom(Route<B>),
}

impl<B> Clone for Fallback<B> {
    fn clone(&self) -> Self {
        match self {
            Fallback::Default(inner) => Fallback::Default(inner.clone()),
            Fallback::Custom(inner) => Fallback::Custom(inner.clone()),
        }
    }
}

impl<B> fmt::Debug for Fallback<B> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Default(inner) => f.debug_tuple("Default").field(inner).finish(),
            Self::Custom(inner) => f.debug_tuple("Custom").field(inner).finish(),
        }
    }
}

impl<B> Fallback<B> {
    fn map<F, B2>(self, f: F) -> Fallback<B2>
    where
        F: FnOnce(Route<B>) -> Route<B2>,
    {
        match self {
            Fallback::Default(inner) => Fallback::Default(f(inner)),
            Fallback::Custom(inner) => Fallback::Custom(f(inner)),
        }
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
