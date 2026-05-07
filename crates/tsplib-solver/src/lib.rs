//! This crate provides a trait for solving the Traveling Salesman Problem (TSP) using various algorithms.
pub mod errors;

use errors::SolverError;
use tsplib_core::models::{ProblemInstance, TspSolution};

pub trait TspSolver {
    fn try_solve(&self, problem: &ProblemInstance) -> Result<TspSolution, SolverError>;
}
