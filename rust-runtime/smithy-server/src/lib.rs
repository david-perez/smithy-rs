use axum::{
    async_trait,
    extract::{FromRequest, RequestParts},
    handler::get,
    response::IntoResponse,
    Router,
};
use derive_builder::Builder;
use hyper::{Body, Response};
use simple::{input, output};
use std::future::Future;
use std::marker::PhantomData;

// Newtypes to impl `IntoResponse` and `FromRequest`.
// These won't be needed in reality because the traits will be code-generated in the `input` and
// `output` crates.
struct HealthcheckOutput(output::HealthcheckOutput);
struct HealthcheckInput(input::HealthcheckInput);
struct RegisterServiceOutput(output::RegisterServiceOutput);
struct RegisterServiceInput(input::RegisterServiceInput);

// ========================================================
// Code-generated by `smithy-rs` in some crate (which one?)
// ========================================================

#[derive(Builder, Debug)]
#[builder(pattern = "owned")]
struct SimpleServiceOperationRegistry<C1, Fut1, C2, Fut2> {
    pub health_check: C1,
    pub register_service: C2,

    _phantom_fut1: PhantomData<Fut1>,
    _phantom_fut2: PhantomData<Fut2>,
}

// Auto-generated depending on Smithy protocol.
// TODO What happens if a Smithy service definition supports more than one protocol?
// This doesn't violate coherence because we control the Cx, Fx type parameters.
impl<C1, Fut1, C2, Fut2> From<SimpleServiceOperationRegistry<C1, Fut1, C2, Fut2>> for Router<axum::routing::BoxRoute>
where
    C1: FnOnce(HealthcheckInput) -> Fut1 + Clone + Send + Sync + 'static,
    Fut1: Future<Output = HealthcheckOutput> + Send,
    C2: FnOnce(RegisterServiceInput) -> Fut2 + Clone + Send + Sync + 'static,
    Fut2: Future<Output = RegisterServiceOutput> + Send,
{
    fn from(registry: SimpleServiceOperationRegistry<C1, Fut1, C2, Fut2>) -> Self {
        Router::new()
            .route("/healthcheck", get(registry.health_check))
            .route("/register_service", get(registry.register_service))
            .boxed()
    }
}

// ================================================================
// Code-generated by `smithy-rs` in the `input` and `output` crates
// ================================================================

impl IntoResponse for HealthcheckOutput {
    type Body = axum::body::Body;
    type BodyError = <Self::Body as axum::body::HttpBody>::Error;

    fn into_response(self) -> Response<Self::Body> {
        Response::builder()
            .body(Body::from(String::from("output::HealthcheckOutput has no fields, but we would read them here")))
            .unwrap()
    }
}

#[async_trait]
impl<B> FromRequest<B> for HealthcheckInput
where
    B: Send, // required by `async_trait`
{
    // Or anything that implements `IntoResponse`.
    // TODO Should this be the SmithyError? Not really; rather, a deserialization-specific error
    // provided by the framework.
    type Rejection = http::StatusCode;

    async fn from_request(_req: &mut RequestParts<B>) -> Result<Self, Self::Rejection> {
        // TODO Why do builders for input structs return `Result<T, E>` but those for output
        // structs return `Result<T>`?
        Ok(HealthcheckInput(input::HealthcheckInput::builder().build().unwrap()))
    }
}

// Same thing for the other operation.

impl IntoResponse for RegisterServiceOutput {
    type Body = axum::body::Body;
    type BodyError = <Self::Body as axum::body::HttpBody>::Error;

    fn into_response(self) -> Response<Self::Body> {
        Response::builder().body(Body::from(String::from("RegisterServiceOutput TODO"))).unwrap()
    }
}

#[async_trait]
impl<B> FromRequest<B> for RegisterServiceInput
where
    B: Send,
{
    type Rejection = http::StatusCode;

    async fn from_request(_req: &mut RequestParts<B>) -> Result<Self, Self::Rejection> {
        Ok(RegisterServiceInput(input::RegisterServiceInput::builder().build().unwrap()))
    }
}

// ====================
// What the user writes
// ====================

async fn healthcheck_operation(_input: HealthcheckInput) -> HealthcheckOutput {
    HealthcheckOutput(output::HealthcheckOutput::builder().build())
}

async fn register_service_operation(_input: RegisterServiceInput) -> RegisterServiceOutput {
    RegisterServiceOutput(output::RegisterServiceOutput::builder().id(String::from("id")).build())
}

#[tokio::main]
async fn main() {
    let app: Router<axum::routing::BoxRoute> = SimpleServiceOperationRegistryBuilder::default()
        // User builds a registry containing implementations to all the operations in the service.
        // These are async functions or async closures that take as input the operation's
        // input and return the operation's output.
        .health_check(healthcheck_operation)
        .register_service(register_service_operation)
        .build()
        .unwrap()
        // Convert it into an axum router that will route requests to the matching operation
        // implementation.
        .into();

    // User has the ability to modify app if they desire.
    // They can add layers to **all** routes.
    // TODO How can they add layers per route? They can't modify the routes in the router to wrap
    // them in https://docs.rs/axum/0.2.8/axum/handler/trait.Handler.html#method.layer

    let server = axum::Server::bind(&"0.0.0.0:8080".parse().unwrap()).serve(app.into_make_service());

    // Run forever-ish...
    if let Err(err) = server.await {
        eprintln!("server error: {}", err);
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let val = 4;
        assert_eq!(val, 4);
    }
}
