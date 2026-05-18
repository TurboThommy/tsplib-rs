//! Handlers for the REST API routes related to running the solvers.
use std::fs;

use axum::{Json, Router, extract::State, http::StatusCode, routing::post};
use tokio_util::sync::CancellationToken;
use tsplib_core::{
    context::ExecutionContext,
    enums::AlgorithmType,
    models::{ProblemInstance, TspSolution},
};
use tsplib_parser::try_parse;
use tsplib_solver::{Greedy, HeldKarp, TspSolver};

use crate::{
    errors::ServerError,
    models::requests::StartSolverRequest,
    state::{AppState, SolverState},
};

/// Router for solver-related endpoints.
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/solver/start", post(start_solver))
        .route("/solver/cancel", post(cancel_solver))
}

/// Internal helper function to run the solver in a blocking task.
/// This function reads the problem data from a file, parses it, and runs the selected algorithm.
///
/// # Arguments
/// * `algorithm` - The algorithm to use for solving the TSP problem.
/// * `problem_id` - The ID of the problem instance to solve.
/// * `start_node` - Optional starting node for the TSP tour.
///
/// # Returns
/// * `Result<TspSolution, ServerError>` - The solution to the TSP problem or an error if something goes wrong.
fn run_solver(
    algorithm: AlgorithmType,
    problem_id: String,
    start_node: Option<usize>,
    token: CancellationToken,
) -> Result<TspSolution, ServerError> {
    // create a cancellation function that checks if the token has been cancelled
    let cancellation = || token.is_cancelled();
    let ctx = ExecutionContext::new(&cancellation);

    // define file path to the problem instance
    let path = format!("./data/{}.tsp", problem_id);

    // read file and parse as ProblemInstance
    let problem_data = fs::read_to_string(path);
    let problem: ProblemInstance = match problem_data {
        Ok(data) => try_parse(data)?.try_into_problem_instance(ctx)?, // parse data into ProblemInstance
        Err(_) => return Err(ServerError::ProblemNotFound(problem_id)),
    };

    // check cancellation before starting the solver
    if ctx.is_cancelled() {
        return Err(ServerError::SolverCancelled);
    }

    let solver: Box<dyn TspSolver> = match algorithm {
        AlgorithmType::Greedy => Box::new(Greedy::new()),
        AlgorithmType::HeldKarp => Box::new(HeldKarp::try_new(25)?),
    };

    Ok(solver.try_solve_with_context(&problem, start_node.unwrap_or(1), ctx)?)
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
) -> Result<Json<TspSolution>, ServerError> {
    // check if a solver is already running
    let mut solver_state = state.solver_state.lock().await;

    if let SolverState::Processing(_) = *solver_state {
        return Err(ServerError::SolverAlreadyRunning);
    }

    // create a cancellation token for the new solver task
    let token = CancellationToken::new();
    // create a second handle for the cancellation token to pass to the solver task
    let task_token = token.clone();

    // spawn a blocking task to run the solver and set the solver state to processing
    let handle = tokio::task::spawn_blocking(move || {
        run_solver(
            request.algorithm,
            request.problem_id,
            request.start_node,
            task_token,
        )
    });

    *solver_state = SolverState::Processing(token);
    drop(solver_state);

    // wait for the solver to finish and get the result
    let result = handle.await;

    // reset solver state to idle after completion
    *state.solver_state.lock().await = SolverState::Idle;

    match result {
        Ok(Ok(solution)) => Ok(Json(solution)),
        Ok(Err(e)) => Err(e),
        Err(e) if e.is_cancelled() => Err(ServerError::SolverCancelled),
        Err(e) => Err(ServerError::from(e)),
    }
}

/// Cancels the currently running TSP solver, if any.
/// If a solver is running, it aborts the task and resets the solver state to idle.
/// If no solver is running, it returns a bad request status code.
///
/// # Arguments
/// * `State(state)` - The shared application state containing the solver state.
///
/// # Returns
/// * `StatusCode` - HTTP status code indicating the result of the cancellation attempt.
async fn cancel_solver(State(state): State<AppState>) -> StatusCode {
    let solver_state = state.solver_state.lock().await;

    if let SolverState::Processing(ct) = &*solver_state {
        ct.cancel();
        StatusCode::OK
    } else {
        StatusCode::BAD_REQUEST
    }
}
