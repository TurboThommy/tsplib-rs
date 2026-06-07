//! Handlers for the REST API routes related to known solutions of TSPLIB problem instances.

use axum::{
    Json, Router,
    extract::{Path, State},
    routing::get,
};

use crate::{errors::ServerError, models::responses::SolutionResponse, state::AppState};

/// Router for solution-related endpoints.
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/solutions", get(get_solutions))
        .route("/solutions/{problemId}", get(get_solution))
}

/// Get the list of known solution costs for all problem instances.
///
/// # Returns
/// * `Json<Vec<SolutionResponse>>` - A list of problem IDs and their known solution costs in JSON format.
pub async fn get_solutions(State(state): State<AppState>) -> Json<Vec<SolutionResponse>> {
    let mut all: Vec<SolutionResponse> = state
        .solutions
        .iter()
        .map(|(id, &cost)| SolutionResponse {
            id: id.clone(),
            cost,
        })
        .collect();

    all.sort_by(|a, b| a.id.cmp(&b.id));

    Json(all)
}

/// Get the known solution cost for a specific problem instance by its ID.
///
/// # Arguments
/// * `id` - The ID of the problem instance to retrieve the solution cost for.
///
/// # Returns
/// * `Json<SolutionResponse>` - The solution cost for the specified problem instance in JSON format
///   or an error if the problem ID is not found in the known solutions.
pub async fn get_solution(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<SolutionResponse>, ServerError> {
    match state.solutions.get(&id) {
        Some(&cost) => Ok(Json(SolutionResponse { id, cost })),
        None => Err(ServerError::SolutionProblemIdNotFound(id.to_string())),
    }
}
