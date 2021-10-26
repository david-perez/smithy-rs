// ====================
// What the user writes
// ====================

use aws_smithy_server::model::*;
use aws_smithy_server::operation_registry::SimpleServiceOperationRegistryBuilder;
use aws_smithy_server::runtime::AwsRestJson1;
use axum::Router;
use simple::output;

// Notice how this operation implementation does not return `Result<T, E>`, because the Smithy
// model declares this operation as being infallible (no `errors` property:
// https://awslabs.github.io/smithy/1.0/spec/core/model.html#operation).
async fn healthcheck_operation(_input: HealthcheckInput) -> HealthcheckOutput {
    HealthcheckOutput(output::HealthcheckOutput::builder().build())
}

async fn register_service_operation(
    _input: AwsRestJson1<RegisterServiceInput>,
) -> Result<RegisterServiceOutput, RegisterServiceError> {
    Ok(RegisterServiceOutput(output::RegisterServiceOutput::builder().id(String::from("id")).build()))
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
