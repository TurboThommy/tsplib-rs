//! Handlers for the REST API routes related to problem instances.

use crate::{
    errors::ServerError,
    models::responses::{ProblemDescriptionResponse, TsplibInstanceWithMatrixResponse},
    state::AppState,
};

use axum::{
    Json, Router,
    extract::{Path, State},
    routing::get,
};

/// Router for problem-related endpoints.
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/problems", get(get_problems))
        .route("/problems/{problemId}", get(get_problem))
        .route(
            "/problems/{problemId}/adjacency_matrix",
            get(get_adjacency_matrix),
        )
}

/// Get the list of available TSP problem instances from the "./data" directory.
///
/// # Arguments
/// * `State(state): State<AppState>` - The shared application state containing the preloaded problem instances and their metadata.
///
/// # Returns
/// * `Json<Vec<String>>` - A list of problem IDs (filenames without extension) in JSON format
///   or an error if the directory cannot be read.
async fn get_problems(
    State(state): State<AppState>,
) -> Result<Json<Vec<ProblemDescriptionResponse>>, ServerError> {
    tracing::info!("Received request to get list of problems");

    // read all tsp files from the "./data" directory
    // filter relevant metadata
    tracing::debug!("Reading TSP files from ./data directory");

    // use the preloaded instances from the app state to avoid reading and parsing the files again
    let problems = state
        .instances
        .values()
        .map(|i| i.try_into())
        .collect::<Result<Vec<_>, _>>()?;

    tracing::info!(
        problem_count = problems.len(),
        "Successfully read problems from ./data directory"
    );

    Ok(Json(problems))
}

/// Get a specific TSP problem instance by its ID.
///
/// # Arguments
/// * `problem_id` - The ID of the problem instance to retrieve (corresponds to the filename without extension).
///
/// # Returns
/// * `Json<ProblemInstance>` - The problem instance data in JSON format or an error
///   if the problem is not found or if another processing task is currently running.
async fn get_problem(
    State(state): State<AppState>,
    Path(problem_id): Path<String>,
) -> Result<Json<TsplibInstanceWithMatrixResponse>, ServerError> {
    tracing::info!(problem_id = %problem_id, state = ?state.solver_state, "Received request to get problem instance");

    let instance = state
        .get_instance(&problem_id)
        .ok_or(ServerError::ProblemInstanceNotFound(problem_id.clone()))?;

    let response = state
        .run_cancellable(move |ctx| {
            tracing::debug!(problem_id = %instance.problem_id, "Attempting to create response with full adjacency matrix");
            TsplibInstanceWithMatrixResponse::try_from_instance(&instance, ctx)
        })
        .await?;

    Ok(Json(response))
}

/// Get the full adjacency matrix for a specific TSP problem instance by its ID.
///
/// # Arguments
/// * `problem_id` - The ID of the problem instance to retrieve the adjacency matrix for (corresponds to the filename without extension).
///
/// # Returns
/// * `Json<Vec<Vec<i32>>>` - The adjacency matrix of the problem instance in JSON format or an error if the problem is not found,
///   if another processing task is currently running, or if the processing task was cancelled.
#[allow(clippy::needless_range_loop)]
async fn get_adjacency_matrix(
    State(state): State<AppState>,
    Path(problem_id): Path<String>,
) -> Result<Json<Vec<Vec<i32>>>, ServerError> {
    tracing::info!(problem_id = %problem_id, state = ?state.solver_state, "Received request to get full adjacency matrix for problem instance");

    let instance = state
        .get_instance(&problem_id)
        .ok_or(ServerError::ProblemInstanceNotFound(problem_id.clone()))?;

    let matrix = state
        .run_cancellable(move |ctx| {
            tracing::debug!(problem_id = %instance.problem_id, "Attempting to create full adjacency matrix");
        let n = instance.nodes.len();
        let mut adjacency_matrix = vec![vec![0; n]; n];

        for i in 0..n {
            if ctx.is_cancelled() {
                return Err(ServerError::ProcessingCancelled);
            }

            for j in i + 1..n {
                if i == j {
                    continue;
                }

                let distance = instance.try_get_distance(i + 1, j + 1)?;
                adjacency_matrix[i][j] = distance;
                adjacency_matrix[j][i] = distance;
            }
        }
        Ok(adjacency_matrix)
        })
        .await?;

    Ok(Json(matrix))
}

// TODO: add endpoint to get the full problem without adjacency matrix

// TODO: add endpoint to get a specific edge

// TODO: add endpoint to get all edges for a specific node
