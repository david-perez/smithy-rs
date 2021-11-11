use axum::response::IntoResponse;
use http::Response;
use hyper::Body;

use crate::model::{HealthcheckOutput, RegisterServiceError, RegisterServiceOutput};

// Operation output types.

impl IntoResponse for HealthcheckOutput {
    type Body = axum::body::Body;
    type BodyError = <Self::Body as axum::body::HttpBody>::Error;

    fn into_response(self) -> Response<Self::Body> {
        Response::builder()
            .body(Body::from(String::from(
                "output::HealthcheckOutput has no fields, but we would read them here",
            )))
            .unwrap()
    }
}

impl IntoResponse for RegisterServiceOutput {
    type Body = axum::body::Body;
    type BodyError = <Self::Body as axum::body::HttpBody>::Error;

    fn into_response(self) -> Response<Self::Body> {
        Response::builder()
            .body(Body::from(String::from("RegisterServiceOutput TODO")))
            .unwrap()
    }
}

// Operation error types.

impl IntoResponse for RegisterServiceError {
    type Body = axum::body::Body;
    type BodyError = <Self::Body as axum::body::HttpBody>::Error;

    fn into_response(self) -> Response<Self::Body> {
        todo!()
    }
}
