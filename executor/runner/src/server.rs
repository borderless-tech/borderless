use anyhow::Result;
use axum::{
    body::Body,
    http::{Request, Response},
    Router,
};
use std::future::{ready, Ready};
use std::{
    convert::Infallible,
    task::{Context, Poll},
};
use tower::Service;

#[derive(Clone)]
struct MyService;

impl Service<Request<Body>> for MyService {
    type Response = Response<Body>;
    type Error = Infallible;
    type Future = Ready<Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: Request<Body>) -> Self::Future {
        let uri = req.uri().to_string();
        let response = Response::new(Body::from(format!("Handled by MyService: {}", uri)));
        ready(Ok(response))
    }
}

pub async fn start_contract_server() -> Result<()> {
    let srv = MyService;

    // Create a router and attach the custom service to a route
    let app = Router::new().route_service("/", srv);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .unwrap();

    axum::serve(listener, app).await?;
    Ok(())
}
