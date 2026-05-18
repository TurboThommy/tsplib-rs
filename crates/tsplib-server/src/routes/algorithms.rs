//! Handlers for the REST API routes related to solver algorithms.
use axum::{Json, Router, routing::get};
use strum::IntoEnumIterator;
use tsplib_core::enums::AlgorithmType;

use crate::state::AppState;

/// Router for algorithm-related endpoints.
pub fn router() -> Router<AppState> {
    Router::new().route("/algorithms", get(get_algorithms))
}

/// Get the list of available algorithms.
async fn get_algorithms() -> Json<Vec<AlgorithmType>> {
    Json(AlgorithmType::iter().collect())
}
