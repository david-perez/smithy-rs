use std::sync::Arc;

use http::Uri;
use hyper::client::connect::Connection;

use tokio::io::{AsyncRead, AsyncWrite};
use tower::Service;

use smithy_async::rt::sleep::{AsyncSleep, TokioSleep};
use smithy_http::body::SdkBody;
pub use smithy_http::result::{SdkError, SdkSuccess};

use crate::hyper_impls::timeout_middleware::{ConnectTimeout, HttpReadTimeout};
use crate::{timeout, BoxError, Builder as ClientBuilder};

/// Adapter from a [`hyper::Client`] to a connector usable by a [`Client`](crate::Client).
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct HyperAdapter<C>(HttpReadTimeout<hyper::Client<ConnectTimeout<C>, SdkBody>>);

impl<C> Service<http::Request<SdkBody>> for HyperAdapter<C>
where
    C: Clone + Send + Sync + 'static,
    C: tower::Service<Uri>,
    C::Response: Connection + AsyncRead + AsyncWrite + Send + Unpin + 'static,
    C::Future: Unpin + Send + 'static,
    C::Error: Into<BoxError>,
{
    type Response = http::Response<SdkBody>;
    type Error = BoxError;

    #[allow(clippy::type_complexity)]
    type Future = std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<Self::Response, Self::Error>> + Send + 'static>,
    >;

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.0.poll_ready(cx)
    }

    fn call(&mut self, req: http::Request<SdkBody>) -> Self::Future {
        let fut = self.0.call(req);
        Box::pin(async move { Ok(fut.await?.map(SdkBody::from)) })
    }
}

impl HyperAdapter<()> {
    /// Builder for a Hyper Adapter
    ///
    /// Generally, end users should not need to construct a HyperAdapter manually: a hyper adapter
    /// will be constructed automatically during client creation.
    pub fn builder() -> Builder {
        Builder::default()
    }
}

#[derive(Default, Debug)]
/// Builder for [`HyperAdapter`]
pub struct Builder {
    timeout: timeout::Config,
    sleep: Option<Arc<dyn AsyncSleep>>,
    client_builder: hyper::client::Builder,
}

impl Builder {
    /// Create a HyperAdapter from this builder and a given connector
    pub fn build<C>(self, connector: C) -> HyperAdapter<C>
    where
        C: Clone + Send + Sync + 'static,
        C: tower::Service<Uri>,
        C::Response: Connection + AsyncRead + AsyncWrite + Send + Unpin + 'static,
        C::Future: Unpin + Send + 'static,
        C::Error: Into<BoxError>,
    {
        let sleep = self.sleep.unwrap_or_else(|| Arc::new(TokioSleep::new()));
        let connector = match self.timeout.connect() {
            Some(duration) => ConnectTimeout::new(connector, sleep.clone(), duration),
            None => ConnectTimeout::no_timeout(connector),
        };
        let base = self.client_builder.build(connector);
        let http_timeout = match self.timeout.read() {
            Some(duration) => HttpReadTimeout::new(base, sleep, duration),
            None => HttpReadTimeout::no_timeout(base),
        };
        HyperAdapter(http_timeout)
    }

    /// Set the async sleep implementation used for timeouts
    pub fn sleep_impl(self, sleep_impl: impl AsyncSleep + 'static) -> Self {
        Self {
            sleep: Some(Arc::new(sleep_impl)),
            ..self
        }
    }

    /// Configure the timeout for the HyperAdapter
    ///
    /// When unset, this defaults to [`TokioSleep`], a sleep implementation that uses Tokio.
    pub fn timeout(self, timeout_config: &timeout::Config) -> Self {
        Self {
            timeout: timeout_config.clone(),
            ..self
        }
    }

    /// Override the Hyper client [`Builder`](hyper::client::Builder) used to construct this client.
    ///
    /// This enables changing settings like forcing HTTP2 and modifying other default client behavior.
    pub fn hyper_builder(self, hyper_builder: hyper::client::Builder) -> Self {
        Self {
            client_builder: hyper_builder,
            ..self
        }
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
        let with_https = |b: ClientBuilder<_>| b.rustls();
        // If we are compiling this function & rustls is not enabled, then native-tls MUST be enabled
        #[cfg(not(feature = "rustls"))]
        let with_https = |b: ClientBuilder<_>| b.native_tls();

        with_https(ClientBuilder::new())
            .middleware(M::default())
            .build()
            .into_dyn_connector()
    }
}

#[cfg(feature = "rustls")]
impl<M, R> ClientBuilder<(), M, R> {
    /// Connect to the service over HTTPS using Rustls.
    pub fn rustls(self) -> ClientBuilder<HyperAdapter<crate::conns::Https>, M, R> {
        self.connector(
            HyperAdapter::builder()
                .sleep_impl(TokioSleep::new())
                .build(crate::conns::https()),
        )
    }

    /// Connect to the service over HTTPS using Rustls.
    ///
    /// This is exactly equivalent to [`Builder::rustls`]. If you instead wish to use `native_tls`,
    /// use `Builder::native_tls`.
    pub fn https(self) -> ClientBuilder<HyperAdapter<crate::conns::Https>, M, R> {
        self.rustls()
    }
}
#[cfg(feature = "native-tls")]
impl<M, R> ClientBuilder<(), M, R> {
    /// Connect to the service over HTTPS using the native TLS library on your platform.
    pub fn native_tls(
        self,
    ) -> ClientBuilder<HyperAdapter<hyper_tls::HttpsConnector<hyper::client::HttpConnector>>, M, R>
    {
        self.connector(
            HyperAdapter::builder()
                .sleep_impl(Arc::new(TokioSleep::new()))
                .build(crate::conns::native_tls()),
        )
    }
}

mod timeout_middleware {
    use std::future::Future;
    use std::pin::Pin;
    use std::sync::Arc;
    use std::task::{Context, Poll};
    use std::time::Duration;

    use http::Uri;

    use pin_project_lite::pin_project;

    use smithy_async::future;
    use smithy_async::future::timeout::Timeout;
    use smithy_async::rt::sleep::AsyncSleep;
    use smithy_async::rt::sleep::Sleep;

    use crate::BoxError;

    /// Timeout wrapper that will timeout on the initial TCP connection
    ///
    /// # Stability
    /// This interface is unstable.
    #[derive(Clone, Debug)]
    pub(super) struct ConnectTimeout<I> {
        inner: I,
        timeout: Option<(Arc<dyn AsyncSleep>, Duration)>,
    }

    impl<I> ConnectTimeout<I> {
        /// Create a new `ConnectTimeout` around `inner`.
        ///
        /// Typically, `I` will implement [`hyper::client::connect::Connect`].
        pub fn new(inner: I, sleep: Arc<dyn AsyncSleep>, timeout: Duration) -> Self {
            Self {
                inner,
                timeout: Some((sleep, timeout)),
            }
        }

        pub fn no_timeout(inner: I) -> Self {
            Self {
                inner,
                timeout: None,
            }
        }
    }

    #[derive(Clone, Debug)]
    pub struct HttpReadTimeout<I> {
        inner: I,
        timeout: Option<(Arc<dyn AsyncSleep>, Duration)>,
    }

    impl<I> HttpReadTimeout<I> {
        /// Create a new `HttpReadTimeout` around `inner`.
        ///
        /// Typically, `I` will implement [`tower::Service<http::Request<SdkBody>>`].
        pub fn new(inner: I, sleep: Arc<dyn AsyncSleep>, timeout: Duration) -> Self {
            Self {
                inner,
                timeout: Some((sleep, timeout)),
            }
        }

        pub fn no_timeout(inner: I) -> Self {
            Self {
                inner,
                timeout: None,
            }
        }
    }

    pin_project! {
        #[project = ConnectTimeoutProj]
        pub enum ConnectTimeoutFuture<F> {
            Timeout {
                #[pin]
                timeout: Timeout<F, Sleep>
            },
            NoTimeout {
                #[pin]
                future: F
            }
        }
    }

    impl<F, T, E> Future for ConnectTimeoutFuture<F>
    where
        F: Future<Output = Result<T, E>>,
        E: Into<BoxError>,
    {
        type Output = Result<T, BoxError>;

        fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
            let timout_fut = match self.project() {
                ConnectTimeoutProj::NoTimeout { future } => {
                    return future.poll(cx).map_err(|err| err.into())
                }
                ConnectTimeoutProj::Timeout { timeout } => timeout,
            };
            match timout_fut.poll(cx) {
                Poll::Ready(Ok(no_timeout)) => Poll::Ready(no_timeout.map_err(|err| err.into())),
                Poll::Ready(Err(e)) => Poll::Ready(Err(e.into())),
                Poll::Pending => Poll::Pending,
            }
        }
    }

    impl<I> tower::Service<Uri> for ConnectTimeout<I>
    where
        I: tower::Service<Uri>,
        I::Error: Into<BoxError>,
    {
        type Response = I::Response;
        type Error = BoxError;
        type Future = ConnectTimeoutFuture<I::Future>;

        fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
            self.inner.poll_ready(cx).map_err(|err| err.into())
        }

        fn call(&mut self, req: Uri) -> Self::Future {
            match &self.timeout {
                Some((sleep, duration)) => {
                    let sleep = sleep.sleep(*duration);
                    ConnectTimeoutFuture::Timeout {
                        timeout: future::timeout::Timeout::new(self.inner.call(req), sleep),
                    }
                }
                None => ConnectTimeoutFuture::NoTimeout {
                    future: self.inner.call(req),
                },
            }
        }
    }

    impl<I, B> tower::Service<http::Request<B>> for HttpReadTimeout<I>
    where
        I: tower::Service<http::Request<B>>,
        I::Error: Into<BoxError>,
    {
        type Response = I::Response;
        type Error = BoxError;
        type Future = ConnectTimeoutFuture<I::Future>;

        fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
            self.inner.poll_ready(cx).map_err(|err| err.into())
        }

        fn call(&mut self, req: http::Request<B>) -> Self::Future {
            match &self.timeout {
                Some((sleep, duration)) => {
                    let sleep = sleep.sleep(*duration);
                    ConnectTimeoutFuture::Timeout {
                        timeout: future::timeout::Timeout::new(self.inner.call(req), sleep),
                    }
                }
                None => ConnectTimeoutFuture::NoTimeout {
                    future: self.inner.call(req),
                },
            }
        }
    }

    #[cfg(test)]
    mod test {

        #[allow(unused)]
        fn connect_timeout_is_correct<T: Send + Sync + Clone + 'static>() {
            is_send_sync::<super::ConnectTimeout<T>>();
        }

        #[allow(unused)]
        fn is_send_sync<T: Send + Sync>() {}
    }
}
