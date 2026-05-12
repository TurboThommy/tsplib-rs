//! REST API for the TSP library.

use axum::{Json, Router, http::StatusCode, routing::get};
use strum::IntoEnumIterator;
use tsplib_core::enums::AlgorithmType;

#[tokio::main]
async fn main() {
    let app = Router::new()
        .route("/", get(health_check))
        .route("/algorithms", get(get_algorithms));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();

    println!("Listening on http://{}", listener.local_addr().unwrap());

    axum::serve(listener, app).await.unwrap();
}

/// Health check endpoint.
async fn health_check() -> StatusCode {
    StatusCode::OK
}

/// Get the list of available algorithms.
async fn get_algorithms() -> Json<Vec<AlgorithmType>> {
    Json(AlgorithmType::iter().collect())
}
