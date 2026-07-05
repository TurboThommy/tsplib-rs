//! Handlers for the REST API routes related to solvers.
use std::time::Instant;

use axum::{
    Json, Router,
    extract::State,
    routing::{get, post},
};
use strum::IntoEnumIterator;
use tsplib_core::{context::ExecutionContext, models::TspSolution, reader::try_read_tsp_file};
use tsplib_parser::try_parse;
use tsplib_solver::{
    Christofides, Greedy, HeldKarp, LinearProgram, LpOptimized, SolverOptions, TspSolver,
    enums::SolverAlgorithm::{self},
};

use crate::{
    errors::ServerError,
    models::{requests::StartSolverRequest, responses::StartSolverResponse},
    state::AppState,
};

/// Router for solver-related endpoints.
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/solver/start", post(start_solver))
        .route("/solver/algorithms", get(get_algorithms))
}

/// Internal helper function to run the solver in a blocking task.
/// This function reads the problem data from a file, parses it, and runs the selected algorithm.
///
/// # Arguments
/// * `algorithm` - The algorithm to use for solving the TSP problem.
/// * `problem_id` - The ID of the problem instance to solve    .
/// * `start_node` - Optional starting node for the TSP tour.
/// * `token` - A cancellation token to allow for cancelling the solver task.
/// * `options` - Additional options for the solver.
///
/// # Returns
/// * `Result<TspSolution, ServerError>` - The solution to the TSP problem or an error if something goes wrong.
fn run_solver(
    algorithm: SolverAlgorithm,
    problem_id: String,
    start_node: Option<usize>,
    ctx: &ExecutionContext,
    options: SolverOptions,
) -> Result<TspSolution, ServerError> {
    tracing::debug!(
        algorithm = ?algorithm,
        problem_id = %problem_id,
        start_node = ?start_node,
        solver_options = ?options,
        "Running solver with provided parameters"
    );

    // define file path to the problem instance
    let path = format!("./data/{}.tsp", problem_id);

    // read file and parse as ProblemInstance
    tracing::debug!("Attempting to read and parse problem instance");
    let (problem_id, problem_data) = try_read_tsp_file(path.as_ref())?;
    let problem = try_parse(problem_id, problem_data)?.try_into_problem_instance(*ctx)?;

    // check cancellation before starting the solver
    if ctx.is_cancelled() {
        return Err(ServerError::ProcessingCancelled);
    }

    let solver: Box<dyn TspSolver> = match algorithm {
        SolverAlgorithm::Greedy => Box::new(Greedy::new()),
        SolverAlgorithm::HeldKarp => Box::new(HeldKarp::try_new(25)?),
        SolverAlgorithm::Christofides => Box::new(Christofides::new()),
        SolverAlgorithm::LinearProgramming => Box::new(LinearProgram::new()),
        SolverAlgorithm::LpOptimized => Box::new(LpOptimized::new()),
    };
    tracing::debug!(solver_algorithm = ?algorithm, "Initialized solver instance, trying to solve problem");

    let solution =
        solver.try_solve_with_context(&problem, start_node.unwrap_or(1), *ctx, options)?;

    tracing::debug!(tour_weight = %solution.cost, "Solver completed successfully");

    Ok(solution)
}

/// Starts the TSP solver for a given problem instance and algorithm.
/// Checks if a solver is already running and returns an error if so.
/// Otherwise, it spawns a blocking task to run the solver and returns the solution once it's done.
///
/// # Arguments
/// * `State(state)` - The shared application state containing the solver state.
/// * `Json(request)` - The JSON payload containing the algorithm, problem ID, and optional starting node.
///
/// # Returns
/// * `Result<Json<TspSolution>, ServerError>` - The TSP solution in JSON format or an error if something goes wrong.
async fn start_solver(
    State(state): State<AppState>,
    Json(request): Json<StartSolverRequest>,
) -> Result<Json<StartSolverResponse>, ServerError> {
    tracing::info!(request = ?request, "Received request to start solver");

    // get solver options from the request or use default if not provided
    let solver_options = request.solver_options.unwrap_or_default();

    tracing::debug!(
        solver_algorithm = ?request.algorithm,
        problem_id = %request.problem_id,
        start_node = ?request.start_node,
        solver_options = ?solver_options,
        "Starting solver task"
    );

    // start timer for the solver task
    let start_time = Instant::now();

    // run the solver in a cancellable manner
    let solution = state
        .run_cancellable(move |ctx| {
            run_solver(
                request.algorithm,
                request.problem_id,
                request.start_node,
                ctx,
                solver_options,
            )
        })
        .await?;
    let elapsed_time = start_time.elapsed();

    tracing::info!("Solver task completed successfully");
    Ok(Json(StartSolverResponse::from_solution(
        &solution,
        elapsed_time,
    )))
}

/// Get the list of available solver algorithms.
async fn get_algorithms() -> Json<Vec<SolverAlgorithm>> {
    tracing::info!("Received request to get list of available solver algorithms");
    Json(SolverAlgorithm::iter().collect())
}
