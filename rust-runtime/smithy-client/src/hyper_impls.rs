use crate::Builder;
use smithy_http::body::SdkBody;
use smithy_http::result::ClientError;
pub use smithy_http::result::{SdkError, SdkSuccess};
use std::error::Error;
use tower::Service;

/// Adapter from a [`hyper::Client`] to a connector useable by a [`Client`](crate::Client).
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct HyperAdapter<C>(hyper::Client<C, SdkBody>);

impl<C> Service<http::Request<SdkBody>> for HyperAdapter<C>
where
    C: hyper::client::connect::Connect + Clone + Send + Sync + 'static,
{
    type Response = http::Response<SdkBody>;
    type Error = ClientError;

    #[allow(clippy::type_complexity)]
    type Future = std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<Self::Response, Self::Error>> + Send + 'static>,
    >;

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.0.poll_ready(cx).map_err(to_client_error)
    }

    fn call(&mut self, req: http::Request<SdkBody>) -> Self::Future {
        let fut = self.0.call(req);
        Box::pin(async move { Ok(fut.await.map_err(to_client_error)?.map(SdkBody::from)) })
    }
}

fn to_client_error(err: hyper::Error) -> ClientError {
    if err.is_timeout() {
        ClientError::timeout(err.into())
    } else if err.is_user() {
        ClientError::user(err.into())
    } else if find_source::<std::io::Error>(&err).is_some() {
        ClientError::io(err.into())
    } else {
        ClientError::other(err.into(), None)
    }
}

fn find_source<'a, E: Error + 'static>(err: &'a (dyn Error + 'static)) -> Option<&'a E> {
    let mut next = Some(err);
    while let Some(err) = next {
        if let Some(matching_err) = err.downcast_ref::<E>() {
            return Some(matching_err);
        }
        next = err.source();
    }
    None
}

impl<C> From<hyper::Client<C, SdkBody>> for HyperAdapter<C> {
    fn from(hc: hyper::Client<C, SdkBody>) -> Self {
        Self(hc)
    }
}

impl<M, R> Builder<(), M, R> {
    /// Connect to the service using the provided `hyper` client.
    pub fn hyper<HC>(self, connector: hyper::Client<HC, SdkBody>) -> Builder<HyperAdapter<HC>, M, R>
    where
        HC: hyper::client::connect::Connect + Clone + Send + Sync + 'static,
    {
        self.connector(HyperAdapter::from(connector))
    }
}

#[cfg(any(feature = "rustls", feature = "native_tls"))]
impl<M> crate::Client<crate::erase::DynConnector, M>
where
    M: Default,
    M: crate::bounds::SmithyMiddleware<crate::erase::DynConnector> + Send + Sync + 'static,
{
    /// Create a Smithy client that uses HTTPS and the [standard retry
    /// policy](crate::retry::Standard) over the default middleware implementation.
    ///
    /// For convenience, this constructor type-erases the concrete TLS connector backend used using
    /// dynamic dispatch. This comes at a slight runtime performance cost. See
    /// [`DynConnector`](crate::erase::DynConnector) for details. To avoid that overhead, use
    /// [`Builder::rustls`] or `Builder::native_tls` instead.
    pub fn https() -> Self {
        #[cfg(feature = "rustls")]
        let with_https = |b: Builder<_>| b.rustls();
        // If we are compiling this function & rustls is not enabled, then native-tls MUST be enabled
        #[cfg(not(feature = "rustls"))]
        let with_https = |b: Builder<_>| b.native_tls();

        with_https(Builder::new())
            .middleware(M::default())
            .build()
            .into_dyn_connector()
    }
}

#[cfg(feature = "rustls")]
impl<M, R> Builder<(), M, R> {
    /// Connect to the service over HTTPS using Rustls.
    pub fn rustls(
        self,
    ) -> Builder<HyperAdapter<hyper_rustls::HttpsConnector<hyper::client::HttpConnector>>, M, R>
    {
        self.connector(crate::conns::https())
    }

    /// Connect to the service over HTTPS using Rustls.
    ///
    /// This is exactly equivalent to [`Builder::rustls`]. If you instead wish to use `native_tls`,
    /// use `Builder::native_tls`.
    pub fn https(
        self,
    ) -> Builder<HyperAdapter<hyper_rustls::HttpsConnector<hyper::client::HttpConnector>>, M, R>
    {
        self.rustls()
    }
}
#[cfg(feature = "native-tls")]
impl<M, R> Builder<(), M, R> {
    /// Connect to the service over HTTPS using the native TLS library on your platform.
    pub fn native_tls(
        self,
    ) -> Builder<HyperAdapter<hyper_tls::HttpsConnector<hyper::client::HttpConnector>>, M, R> {
        self.connector(crate::conns::native_tls())
    }
}
