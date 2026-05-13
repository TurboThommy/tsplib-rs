//! Application state management for the TSP solver server.
use std::sync::Arc;
use tokio::{sync::Mutex, task::AbortHandle};

/// Represents the current state of the TSP solver,which can either be idle or processing a problem instance.
pub enum SolverState {
    Idle,
    Processing(AbortHandle),
}

/// The shared application state for the TSP solver server, containing the current solver state.
#[derive(Clone)]
pub struct AppState {
    pub solver_state: Arc<Mutex<SolverState>>,
}

impl AppState {
    /// Creates a new instance of the application state with the solver state initialized to idle.
    pub fn new() -> Self {
        AppState {
            solver_state: Arc::new(Mutex::new(SolverState::Idle)),
        }
    }
}
