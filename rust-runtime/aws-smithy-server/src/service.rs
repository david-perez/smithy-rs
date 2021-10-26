// use std::pin::Pin;

// use crate::router::Router;
// use http::{Request, Response, StatusCode};
// use hyper::service::Service;
// use std::future::Future;
// use std::task::{Context, Poll};

// pub struct SimpleService {
//     router: Router,
// }

// impl SimpleService {
//     pub fn new(router: Router) -> Self {
//         Self { router }
//     }
// }

// impl Service<Request<Vec<u8>>> for SimpleService {
//     type Response = Response<Vec<u8>>;
//     type Error = http::Error;
//     type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>>>>;

//     fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
//         Poll::Ready(Ok(()))
//     }

//     fn call(&mut self, req: Request<Vec<u8>>) -> Self::Future {
//         // create the body
//         let body: Vec<u8> = "hello, world!\n".as_bytes().to_owned();
//         // Create the HTTP response
//         let resp = Response::builder().status(StatusCode::OK).body(body).expect("Unable to create `http::Response`");

//         // create a response in a future.
//         let fut = async { Ok(resp) };

//         // Return the response as an immediate future
//         Box::pin(fut)
//     }
// }
