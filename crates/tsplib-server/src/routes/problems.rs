//! Handlers for the REST API routes related to problem instances.

use crate::{
    errors::ServerError,
    models::{
        requests::EdgeBetweenRequest,
        responses::{
            ProblemDescriptionResponse, TsplibInstanceResponse, TsplibInstanceWithMatrixResponse,
        },
    },
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
        .route(
            "/problems/{problemId}/no_matrix",
            get(get_problem_without_matrix),
        )
        .route("/problems/{problemId}/edges", get(get_edge))
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

/// Get a specific TSP problem instance by its ID without the adjacency matrix, which can be expensive to compute for large instances.
///
/// # Arguments
/// * `problem_id` - The ID of the problem instance to retrieve.
///
/// # Returns
/// * `Json<TsplibInstanceResponse>` - The problem instance data without the adjacency matrix in JSON format or an error if the problem is not found.
async fn get_problem_without_matrix(
    State(state): State<AppState>,
    Path(problem_id): Path<String>,
) -> Result<Json<TsplibInstanceResponse>, ServerError> {
    tracing::info!(problem_id = %problem_id, "Received request to get problem instance without adjacency matrix");

    let instance = state
        .get_instance(&problem_id)
        .ok_or(ServerError::ProblemInstanceNotFound(problem_id.clone()))?;

    let response = TsplibInstanceResponse {
        problem_id: instance.problem_id.clone(),
        name: instance.name.clone(),
        problem_type: instance.problem_type.clone(),
        nodes: instance.nodes.clone(),
        fixed_edges: instance.fixed_edges.clone(),
    };

    Ok(Json(response))
}

/// Get the cost of a specific edge between two nodes in a TSP problem instance.
///
/// # Arguments
/// * `state` - The shared application state containing the preloaded problem instances and their metadata.
/// * `problem_id` - The ID of the problem instance to query.
/// * `from` - The starting node of the edge to query.
/// * `to` - The ending node of the edge to query.
///
/// # Returns
/// * `Json<i32>` - The cost of the edge between the specified nodes in JSON format or an error if the problem instance is not found.
async fn get_edge(
    State(state): State<AppState>,
    Path(problem_id): Path<String>,
    Json(request): Json<EdgeBetweenRequest>,
) -> Result<Json<i32>, ServerError> {
    tracing::info!(
        problem_id = %problem_id,
        from = %request.from,
        to = %request.to,
        "Received request to get distance between two nodes"
    );

    let instance = state
        .get_instance(&problem_id)
        .ok_or(ServerError::ProblemInstanceNotFound(problem_id.clone()))?;

    let distance = instance.try_get_distance(request.from, request.to)?;

    Ok(Json(distance))
}

// TODO: add endpoint to get all edges for a specific node
