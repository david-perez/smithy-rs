use async_trait::async_trait;
use axum::extract::{FromRequest, RequestParts};
use axum::response::IntoResponse;
use http::{Request, Response};
use std::future::Future;

use crate::body::{box_body, BoxBody};

pub(crate) mod sealed {
    #![allow(unreachable_pub, missing_docs, missing_debug_implementations)]

    pub trait HiddenTrait {}
    pub struct Hidden;
    impl HiddenTrait for Hidden {}
}

#[async_trait]
pub trait Handler<B, T, I, O>: Clone + Send + Sized + 'static {
    #[doc(hidden)]
    type Sealed: sealed::HiddenTrait;

    async fn call(self, req: Request<B>) -> Response<BoxBody>;
}

#[async_trait]
#[allow(non_snake_case)]
impl<F, Fut, B, Res, T, I, O> Handler<B, T, I, Res> for F
where
    F: FnOnce(I) -> Fut + Clone + Send + Sync + 'static,
    Fut: Future<Output = O> + Send,
    B: Send + 'static,
    Res: From<O>,
    Res: IntoResponse,
    I: From<T> + Send,
    T: FromRequest<B> + Send,
{
    type Sealed = sealed::Hidden;

    async fn call(self, req: Request<B>) -> Response<BoxBody> {
        let mut req = RequestParts::new(req);

        let wrapper = match T::from_request(&mut req).await {
            Ok(value) => value,
            Err(rejection) => return rejection.into_response().map(box_body),
        };

        let input_inner: I = wrapper.into();

        let output_inner: O = self(input_inner).await;

        let res: Res = output_inner.into();

        res.into_response().map(box_body)
    }
}
