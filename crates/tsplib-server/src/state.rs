//! Application state management for the TSP solver server.
use std::{
    collections::HashMap,
    fs,
    sync::{Arc, RwLock},
};
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;
use tsplib_core::{context::ExecutionContext, models::TsplibInstance, reader::read_tsp_files};
use tsplib_parser::parse;

use crate::errors::ServerError;

/// Represents the current state of the TSP solver,which can either be idle or processing a problem instance.
#[derive(Debug)]
pub enum ProcessingState {
    Idle,
    Processing(CancellationToken),
}

/// The shared application state for the TSP solver server, containing the current solver state.
#[derive(Clone, Debug)]
pub struct AppState {
    pub solver_state: Arc<Mutex<ProcessingState>>,
    pub solutions: Arc<HashMap<String, i64>>,
    pub instances: Arc<RwLock<HashMap<String, Arc<TsplibInstance>>>>,
}

impl AppState {
    /// Creates a new instance of the application state with the solver state initialized to idle.
    pub fn new() -> Self {
        AppState {
            solver_state: Arc::new(Mutex::new(ProcessingState::Idle)),
            solutions: Arc::new(parse_solutions()),
            instances: Arc::new(parse_instances()),
        }
    }

    /// Retrieves a TSP problem instance by its ID from the preloaded instances in the application state.
    ///
    /// # Arguments
    /// * `problem_id` - The ID of the problem instance to retrieve.
    ///
    /// # Returns
    /// * `Option<Arc<TsplibInstance>>` - Some with the requested problem instance if found,
    ///   or None if the problem ID does not exist in the preloaded instances.
    pub fn get_instance(&self, problem_id: &str) -> Option<Arc<TsplibInstance>> {
        self.instances
            .read()
            .expect("instances lock is poisoned")
            .get(problem_id)
            .cloned()
    }

    /// Runs a given processing function in a cancellable manner, ensuring that only one processing task can run at a time.
    ///
    /// # Arguments
    /// * `work` - A closure that takes an ExecutionContext and returns a Result with the processing result or a ServerError.
    ///   This closure will be executed in a blocking task.
    ///
    /// # Returns
    /// * `Result<T, ServerError>` - Ok with the result of the processing function if it completes successfully,
    ///   or an Err with a ServerError if an error occurs, if another processing task is already running, or if the processing task was cancelled.
    pub async fn run_cancellable<T, F>(&self, work: F) -> Result<T, ServerError>
    where
        F: FnOnce(&ExecutionContext) -> Result<T, ServerError> + Send + 'static,
        T: Send + 'static,
    {
        // claim processing slot
        let token = CancellationToken::new();
        {
            let mut solver_state = self.solver_state.lock().await;

            if let ProcessingState::Processing(_) = *solver_state {
                return Err(ServerError::ProcessingAlreadyRunning);
            }

            tracing::debug!("Processing task started, updating app state");
            *solver_state = ProcessingState::Processing(token.clone());
            tracing::debug!(state = ?self.solver_state, "App state updated");
        }
        // lock is released here

        tracing::debug!("Starting processing task");
        let handle = tokio::task::spawn_blocking(move || {
            let cancellation = || token.is_cancelled();
            let ctx = ExecutionContext::new(&cancellation);
            work(&ctx)
        });

        // wait for the processing task to complete and get the result
        tracing::debug!("Waiting for processing task to complete");
        let result = handle.await;

        // Always reset state to idle, even on error/cancellation
        tracing::debug!("Processing task completed, resetting solver state to idle");
        *self.solver_state.lock().await = ProcessingState::Idle;
        tracing::debug!(state = ?self.solver_state, "App state updated, returning result");

        match result {
            Ok(Ok(value)) => Ok(value),
            Ok(Err(e)) => Err(e),
            Err(e) if e.is_cancelled() => Err(ServerError::ProcessingCancelled),
            Err(e) => Err(ServerError::from(e)),
        }
    }
}

/// Parses the solutions file from the ./data directory and returns a HashMap of problem IDs to their known solution costs.
///
/// # Returns
/// * `HashMap<String, i64>` - A HashMap where the keys are problem IDs (as Strings) and the values are the known solution costs
///   (as i64) for those problem instances. The function expects the solutions file to be in a specific format where each line
///   contains a problem ID followed by a colon and then the solution cost, e.g. "problem1: 12345".
fn parse_solutions() -> HashMap<String, i64> {
    tracing::info!("Parsing solutions file from ./data directory");

    let content = fs::read_to_string("./data/solutions")
        .expect("Failed to read ./data/solutions (run from workspace root?)");

    let solutions: HashMap<String, i64> = content
        .lines()
        .filter_map(|line| {
            let (name, rest) = line.split_once(':')?;
            let value = rest.split_whitespace().next()?.parse().ok()?;
            Some((name.trim().to_string(), value))
        })
        .collect();

    tracing::info!(
        solutions = solutions.len(),
        "Successfully parsed solution file"
    );

    solutions
}

/// Parses TSP problem instances from the ./data directory and returns a HashMap of problem IDs to their corresponding TsplibInstance
/// wrapped in an Arc for shared ownership.
///
/// # Returns
/// * `HashMap<String, Arc<TsplibInstance>>` - A HashMap where the keys are problem IDs (as Strings) and the values are Arc-wrapped TsplibInstance
///   structs representing the parsed TSP problem instances.
fn parse_instances() -> RwLock<HashMap<String, Arc<TsplibInstance>>> {
    tracing::info!("Parsing TSP instances from ./dat directory");

    let instances = read_tsp_files("./data")
        .into_iter()
        .map(|(problem_id, problem_data)| parse(problem_id, problem_data))
        .flat_map(|def| {
            let problem_id = def.problem_id.clone();
            let result: Result<TsplibInstance, _> = def.try_into();

            match result {
                Ok(instance) => Some(instance),
                Err(e) => {
                    tracing::error!(
                        instance_id = problem_id,
                        error = e.to_string(),
                        "Failed to convert instance to graph representation. Skipping."
                    );
                    None
                }
            }
        })
        .map(|instance| (instance.problem_id.clone(), Arc::new(instance)))
        .collect::<HashMap<String, Arc<TsplibInstance>>>();

    tracing::info!(
        instances = instances.len(),
        "Successfully parsed TSP instances from ./data directory"
    );

    RwLock::new(instances)
}
