// ====================
// What the user writes
// ====================

// use std::sync::Arc;

use aws_smithy_server::operation_registry::SimpleServiceOperationRegistryBuilder;
use aws_smithy_server::runtime::AwsRestJson1;
// use aws_smithy_server::service::SimpleService;
use aws_smithy_server::model::*;
use aws_smithy_server::routing::Router;
// use http::{Request, Response, StatusCode};
// use hyper::service::make_service_fn;
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

// static NOT_FOUND: &[u8] = b"Not Found";

#[tokio::main]
async fn main() {
    let app: Router = SimpleServiceOperationRegistryBuilder::default()
        // Build a registry containing implementations to all the operations in the service.  These
        // are async functions or async closures that take as input the operation's input and
        // return the operation's output.
        .health_check(healthcheck_operation)
        .register_service(register_service_operation)
        .build()
        .unwrap()
        // Convert it into a router that will route requests to the matching operation
        // implementation.
        .into();

    // TODO Allow users to add Tower layers to the `Router`.
    //
    // let make_service = make_service_fn(move |_| {
    //     let router = Arc::clone(&router);

    //     async move {
    //         Ok::<_, std::convert::Infallible>(hyper::service::service_fn(move |mut req| {
    //             let router = router.clone();

    //             async move {
    //                 let out = router.route_and_call(req).await;

    //                 // let out = handler.call(req).await;
    //                 let result: Result<Response<BoxBody>, std::convert::Infallible> = Ok(out);
    //                 result
    //                 // Ok::<_, std::convert::Infallible>(out)

    //                 // Ok::<_, std::convert::Infallible>(
    //                 //     Response::builder().status(StatusCode::NOT_FOUND).body(hyper::Body::from(NOT_FOUND)).unwrap(),
    //                 // )
    //                 // Ok::<_, std::convert::Infallible>(
    //                 //     match router.find(&path) {
    //                 //     Some((handler, params)) => {
    //                 //         let p = params.iter().map(|p| (p.0.to_string(), p.1.to_string())).collect::<Params>();
    //                 //         req.extensions_mut().insert(p);
    //                 //         handler.call(req).await
    //                 //     }
    //                 //     None => Response::builder().status(StatusCode::NOT_FOUND).body(NOT_FOUND.into()).unwrap(),
    //                 // })
    //             }
    //         }))
    //     }
    // });

    let server = axum::Server::bind(&"0.0.0.0:8080".parse().unwrap()).serve(app.into_make_service());

    // Run forever-ish...
    if let Err(err) = server.await {
        eprintln!("server error: {}", err);
    }
}
