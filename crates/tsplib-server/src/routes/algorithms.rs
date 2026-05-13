//! Handlers for the REST API routes related to solver algorithms.
use axum::{Json, Router, routing::get};
use strum::IntoEnumIterator;
use tsplib_core::enums::AlgorithmType;

/// Router for algorithm-related endpoints.
pub fn router() -> Router {
    Router::new().route("/algorithms", get(get_algorithms))
}

/// Get the list of available algorithms.
pub async fn get_algorithms() -> Json<Vec<AlgorithmType>> {
    Json(AlgorithmType::iter().collect())
}
