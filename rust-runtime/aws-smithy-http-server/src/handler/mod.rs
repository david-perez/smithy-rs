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
pub trait Handler<B, T>: Clone + Send + Sized + 'static {
    #[doc(hidden)]
    type Sealed: sealed::HiddenTrait;

    async fn call(self, req: Request<B>) -> Response<BoxBody>;
}

#[async_trait]
impl<F, Fut, Res, B> Handler<B, ()> for F
where
    F: FnOnce() -> Fut + Clone + Send + Sync + 'static,
    Fut: Future<Output = Res> + Send,
    Res: IntoResponse,
    B: Send + 'static,
{
    type Sealed = sealed::Hidden;

    async fn call(self, _req: Request<B>) -> Response<BoxBody> {
        self().await.into_response().map(box_body)
    }
}

macro_rules! impl_handler {
    ( $($ty:ident),* $(,)? ) => {
        #[async_trait]
        #[allow(non_snake_case)]
        impl<F, Fut, B, Res, $($ty,)*> Handler<B, ($($ty,)*)> for F
        where
            F: FnOnce($($ty,)*) -> Fut + Clone + Send + Sync + 'static,
            Fut: Future<Output = Res> + Send,
            B: Send + 'static,
            Res: IntoResponse,
            $( $ty: FromRequest<B> + Send,)*
        {
            type Sealed = sealed::Hidden;

            async fn call(self, req: Request<B>) -> Response<BoxBody> {
                let mut req = RequestParts::new(req);

                $(
                    let $ty = match $ty::from_request(&mut req).await {
                        Ok(value) => value,
                        Err(rejection) => return rejection.into_response().map(box_body),
                    };
                )*

                let res = self($($ty,)*).await;

                res.into_response().map(box_body)
            }
        }
    };
}

impl_handler!(T1);
