//! This module defines the request models for the TSPLIB server.
use serde::Deserialize;
use tsplib_solver::{SolverOptions, enums::SolverAlgorithm};

/// This struct represents the request to start a solver on the TSPLIB server.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StartSolverRequest {
    /// The algorithm to use for solving the problem.
    pub algorithm: SolverAlgorithm,
    /// The ID of the problem to solve.
    pub problem_id: String,
    /// The starting node for the solver, if applicable.
    pub start_node: Option<usize>,
    /// Additional options for the solver, if applicable.
    pub solver_options: Option<SolverOptions>,
}

/// This struct represents the request to get the cost of a specific edge in a TSP problem instance.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EdgeBetweenRequest {
    /// The starting node of the edge to query.
    pub from: usize,
    /// The ending node of the edge to query.
    pub to: usize,
}
