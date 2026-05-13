//! REST API for the TSP library.
mod errors;
mod models;
mod routes;

use axum::{Router, http::StatusCode, routing::get};

#[tokio::main]
async fn main() {
    let app = Router::new()
        .route("/", get(health_check))
        .merge(routes::problems::router())
        .merge(routes::algorithms::router());

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();

    println!("Listening on http://{}", listener.local_addr().unwrap());

    axum::serve(listener, app).await.unwrap();
}

/// Health check endpoint.
async fn health_check() -> StatusCode {
    StatusCode::OK
}
