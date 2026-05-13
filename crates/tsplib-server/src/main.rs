//! REST API for the TSP library.
mod errors;

use errors::ServerError;
use std::fs;

use axum::{Json, Router, http::StatusCode, routing::get};
use strum::IntoEnumIterator;
use tsplib_core::enums::AlgorithmType;

#[tokio::main]
async fn main() {
    let app = Router::new()
        .route("/", get(health_check))
        .route("/algorithms", get(get_algorithms))
        .route("/problems", get(get_problems));

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

/// Get the list of available TSP problem instances from the "./data" directory.
async fn get_problems() -> Result<Json<Vec<String>>, ServerError> {
    let problems = fs::read_dir("./data")?
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let path = entry.path();
            if !path.is_file() {
                return None;
            }
            if path.extension()? != "tsp" {
                return None;
            }
            Some(path.file_stem()?.to_string_lossy().to_string())
        })
        .collect::<Vec<_>>();

    Ok(Json(problems))
}
