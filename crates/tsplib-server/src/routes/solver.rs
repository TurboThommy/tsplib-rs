//! Handlers for the REST API routes related to solvers.
use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Instant,
};

use axum::{
    Json, Router,
    extract::State,
    routing::{get, post},
};
use strum::IntoEnumIterator;
use tokio_util::sync::CancellationToken;
use tsplib_core::{context::ExecutionContext, models::TspSolution, reader::try_read_tsp_file};
use tsplib_parser::try_parse;
use tsplib_solver::{
    Christofides, Greedy, HeldKarp, LinearProgram, LpOptimized, SolverOptions, TspSolver,
    enums::SolverAlgorithm::{self},
};

use crate::{
    errors::ServerError,
    models::{requests::StartSolverRequest, responses::StartSolverResponse},
    monitor,
    state::{AppState, ProcessingState},
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
    token: CancellationToken,
    options: SolverOptions,
) -> Result<TspSolution, ServerError> {
    tracing::debug!(
        algorithm = ?algorithm,
        problem_id = %problem_id,
        start_node = ?start_node,
        solver_options = ?options,
        "Running solver with provided parameters"
    );

    // create a cancellation function that checks if the token has been cancelled
    let cancellation = || token.is_cancelled();
    let ctx = ExecutionContext::new(&cancellation);

    // define file path to the problem instance
    let path = format!("./data/{}.tsp", problem_id);

    // read file and parse as ProblemInstance
    tracing::debug!("Attempting to read and parse problem instance");
    let (problem_id, problem_data) = try_read_tsp_file(path.as_ref())?;
    let problem = try_parse(problem_id, problem_data)?.try_into_problem_instance(ctx)?;

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
        solver.try_solve_with_context(&problem, start_node.unwrap_or(1), ctx, options)?;

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

    // check if a processing task is already running
    let mut solver_state = state.solver_state.lock().await;

    if let ProcessingState::Processing(_) = *solver_state {
        return Err(ServerError::ProcessingAlreadyRunning);
    }

    // get solver options from the request or use default if not provided
    let solver_options = request.solver_options.unwrap_or_default();

    // create a cancellation token for the new solver task
    let token = CancellationToken::new();
    // create a second handle for the cancellation token to pass to the solver task
    let task_token = token.clone();
    // create a third handle for the cancellation token to pass to the resource guard
    let guard_token = token.clone();

    tracing::debug!(
        solver_algorithm = ?request.algorithm,
        problem_id = %request.problem_id,
        start_node = ?request.start_node,
        solver_options = ?solver_options,
        "Starting solver task"
    );

    // start timer for the solver task
    let start_time = Instant::now();

    // spawn a blocking task to run the solver and set the solver state to processing
    let handle = tokio::task::spawn_blocking(move || {
        run_solver(
            request.algorithm,
            request.problem_id,
            request.start_node,
            task_token,
            solver_options,
        )
    });

    let resource_tripped = Arc::new(AtomicBool::new(false));
    let guard = monitor::spawn_memory_guard(
        guard_token,
        monitor::MEMORY_ABORT_THRESHOLD,
        resource_tripped.clone(),
    );

    tracing::debug!("Solver task spawned, updating app state");
    *solver_state = ProcessingState::Processing(token);
    drop(solver_state);
    tracing::debug!(state = ?state.solver_state, "App state updated");

    // wait for the solver to finish and get the result
    tracing::debug!("Waiting for solver task to complete");
    let result = handle.await;

    // solver has returned, stop resource guard
    guard.abort();

    // log the elapsed time for the solver task
    let elapsed_time = start_time.elapsed();

    // reset solver state to idle after completion
    tracing::debug!(elapsed_time = ?elapsed_time, "Solver task completed, resetting solver state to idle");
    *state.solver_state.lock().await = ProcessingState::Idle;
    tracing::debug!(state = ?state.solver_state, "App state updated, returning result");

    match result {
        Ok(Ok(solution)) => {
            tracing::info!("Solver task completed successfully");
            Ok(Json(StartSolverResponse::from_solution(
                &solution,
                elapsed_time,
            )))
        }
        _ if resource_tripped.load(Ordering::SeqCst) => Err(ServerError::ResourceLimitExceeded),
        Ok(Err(e)) => Err(e),
        Err(e) if e.is_cancelled() => Err(ServerError::ProcessingCancelled),
        Err(e) => Err(ServerError::from(e)),
    }
}

/// Get the list of available solver algorithms.
async fn get_algorithms() -> Json<Vec<SolverAlgorithm>> {
    tracing::info!("Received request to get list of available solver algorithms");
    Json(SolverAlgorithm::iter().collect())
}
