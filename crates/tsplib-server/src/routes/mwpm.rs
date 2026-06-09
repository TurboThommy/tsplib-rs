//! Handlers for the REST API routes related to minimum weight perfect matching algorithms.

use axum::{Json, Router, routing::get};
use strum::IntoEnumIterator;

use crate::state::AppState;
use tsplib_solver::enums::MatcherAlgorithm;

/// Router for MWPM-related endpoints.
pub fn router() -> Router<AppState> {
    Router::new().route("/mwpm/algorithms", get(get_algorithms))
}

/// Get the list of available MWPM algorithms.
async fn get_algorithms() -> Json<Vec<MatcherAlgorithm>> {
    tracing::info!(
        "Received request to get list of available minimum weight perfect matching algorithms"
    );
    Json(MatcherAlgorithm::iter().collect())
}
