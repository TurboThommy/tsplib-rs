//! Handlers for the REST API routes related to minimum spanning treee algorithms.
use axum::{Json, Router, routing::get};
use strum::IntoEnumIterator;

use crate::state::AppState;
use tsplib_solver::enums::MstAlgorithm;

/// Router for MST-related endpoints.
pub fn router() -> Router<AppState> {
    Router::new().route("/mst/algorithms", get(get_algorithms))
}

/// Get the list of available MST algorithms.
async fn get_algorithms() -> Json<Vec<MstAlgorithm>> {
    tracing::info!("Received reequest to get list of available minimum spanning tree algorithms");
    Json(MstAlgorithm::iter().collect())
}
