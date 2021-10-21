use std::marker::PhantomData;

use bytes::Bytes;
use async_trait::async_trait;
use axum::{service::get, Router};
use eyre::Result;
use hyper::{Body, Request, Response};
use simple::{operation::{self, Healthcheck}, output};
use tower::limit::ConcurrencyLimitLayer;

#[async_trait]
trait Handler {
    async fn handle(&self, request: Request<bytes::Bytes>) -> Result<Response<Body>>;
}

struct Operation<O> {
    _operation: PhantomData<O>,
}

impl<O> Operation<O> {
    fn new() -> Self {
        Self { _operation: PhantomData }
    }

    async fn run(&self, request: Request<Body>) -> Result<Response<Body>>
    where
        Self: Handler,
    {
        tracing::trace!("raw request {:?}", request);
        let (parts, body) = request.into_parts();
        let request = hyper::body::to_bytes(body).await.map(|bytes| Request::from_parts(parts, bytes))?;
        Ok(self.handle(request).await?)
    }
}

#[async_trait]
impl Handler for Operation<operation::Healthcheck> {
    async fn handle(&self, request: Request<bytes::Bytes>) -> Result<Response<Body>> {
        let input = operation::deser_healthcheck_request(&request)?;
        let output = output::HealthcheckOutput::builder().build();
        Ok(operation::serialize_healthcheck_response(&output).map(|bytes| bytes.map(Body::from))?)
    }
}

async fn run_hc(request: Bytes) -> Result<Bytes> {
    let op = Operation::<Healthcheck>::new();
    Ok(op.run(request).await?)
}

#[tokio::main]
async fn main() {
    let app = Router::new().route("/healthcheck", get(run_hc)).layer(ConcurrencyLimitLayer::new(1000));

    // run it with hyper on localhost:8080
    axum::Server::bind(&"0.0.0.0:8080".parse().unwrap()).serve(app.into_make_service()).await.unwrap();
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let val = 4;
        assert_eq!(val, 4);
    }
}
