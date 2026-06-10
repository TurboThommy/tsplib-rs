//! Application state management for the TSP solver server.
use std::{collections::HashMap, fs, sync::Arc};
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;
use tsplib_core::{models::TsplibInstance, reader::read_tsp_files};
use tsplib_parser::parse;

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
    pub instances: Arc<HashMap<String, Arc<TsplibInstance>>>,
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
        self.instances.get(problem_id).cloned()
    }
}

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

fn parse_instances() -> HashMap<String, Arc<TsplibInstance>> {
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

    instances
}
