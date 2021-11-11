// =============================
// Code-generated by `smithy-rs`
// =============================

use crate::model::*;
use crate::routing::request_spec::{
    PathAndQuerySpec, PathSegment, PathSpec, QuerySegment, UriSpec,
};
use crate::routing::{operation_handler::operation, request_spec::RequestSpec, Router};
use derive_builder::Builder;
use std::future::Future;
// use std::marker::PhantomData;

fn _fun<F, Fut, B, Res, T1>(_: F)
where
    F: FnOnce(T1) -> Fut + Clone + Send + Sync + 'static,
    Fut: Future<Output = Res> + Send,
    B: Send + 'static,
    Res: axum::response::IntoResponse,
    T1: axum::extract::FromRequest<B> + Send,
{
}

#[derive(Builder, Debug)]
#[builder(pattern = "owned")]
pub struct SimpleServiceOperationRegistry<C1, Fut1, C2, Fut2>
where
    C1: FnOnce(HealthcheckInput) -> Fut1 + Clone + Send + Sync + 'static,
    Fut1: Future<Output = HealthcheckOutput> + Send,
    C2: FnOnce(RegisterServiceInput) -> Fut2 + Clone + Send + Sync + 'static,
    Fut2: Future<Output = Result<RegisterServiceOutput, RegisterServiceError>> + Send,
{
    pub health_check: C1,
    pub register_service: C2,
    // We use `PhantomData` here just to not have to specify the trait bounds twice (once in the
    // struct declaration, another in the `impl` block below).
    // However, the `derive_builder` crate does not work with `PhantomData` fields, so `.build().unwrap()`
    // will fail when running the binary. This is therefore commented out. I think this sould be an
    // issue in the `derive_builder` crate.
    // _phantom_fut1: PhantomData<Fut1>,
    // _phantom_fut2: PhantomData<Fut2>,
}

// Auto-generated depending on Smithy protocol.
// TODO What happens if a Smithy service definition supports more than one protocol?
// This doesn't violate coherence because we control the Cx, Futx type parameters.
impl<C1, Fut1, C2, Fut2> From<SimpleServiceOperationRegistry<C1, Fut1, C2, Fut2>> for Router
where
    C1: FnOnce(HealthcheckInput) -> Fut1 + Clone + Send + Sync + 'static,
    Fut1: Future<Output = HealthcheckOutput> + Send,
    C2: FnOnce(RegisterServiceInput) -> Fut2 + Clone + Send + Sync + 'static,
    Fut2: Future<Output = Result<RegisterServiceOutput, RegisterServiceError>> + Send,
{
    fn from(registry: SimpleServiceOperationRegistry<C1, Fut1, C2, Fut2>) -> Self {
        // _fun(registry.register_service);

        // `http localhost:8080/path/to/label/healthcheck`
        let health_check_request_spec = RequestSpec::new(
            http::Method::GET,
            UriSpec {
                host_prefix: None,
                path_and_query: PathAndQuerySpec {
                    path_segments: PathSpec(vec![
                        PathSegment::Literal(String::from("path")),
                        PathSegment::Literal(String::from("to")),
                        PathSegment::Label,
                        PathSegment::Literal(String::from("healthcheck")),
                    ]),
                    query_segments: vec![],
                },
            },
        );

        // `http "localhost:8080/register-service/gre/ee/dy/suffix?key&foo=bar"`
        let register_service_request_spec = RequestSpec::new(
            http::Method::POST,
            UriSpec {
                host_prefix: None,
                path_and_query: PathAndQuerySpec {
                    path_segments: PathSpec(vec![
                        PathSegment::Literal(String::from("register-service")),
                        PathSegment::Greedy,
                        PathSegment::Literal(String::from("suffix")),
                    ]),
                    query_segments: vec![
                        QuerySegment::Key(String::from("key")),
                        QuerySegment::KeyValue(String::from("foo"), String::from("bar")),
                    ],
                },
            },
        );

        // let w = |input: HealthcheckOperationInput| async {
        //     let inner = input.0;
        //     let out = (registry.health_check)(inner).await;
        //     HealthcheckOperationOutput(out)
        // };

        // w.clone();

        // _fun(w);

        // let w = |input: HealthcheckOperationInput| -> Pin<Box<dyn Future<Output = HealthcheckOperationOutput>>> {
        //     let v = async { HealthcheckOperationOutput };

        //     Box::pin(v)
        // };

        Router::new()
            .route(
                health_check_request_spec,
                operation::<_, _, HealthcheckOperationInput, _, HealthcheckOperationOutput>(
                    registry.health_check,
                ),
            )
            .route(
                register_service_request_spec,
                operation::<_, _, RegisterServiceOperationInput, _, RegisterServiceOperationOutput>(
                    registry.register_service,
                ),
            )
    }
}
