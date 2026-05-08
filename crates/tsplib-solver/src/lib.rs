//! This crate provides a trait for solving the Traveling Salesman Problem (TSP) using various algorithms.
pub mod errors;
pub mod greedy;
pub mod held_carp;

pub use greedy::Greedy;
pub use held_carp::HeldCarp;

use errors::SolverError;
use tsplib_core::models::{ProblemInstance, TspSolution};

pub trait TspSolver {
    fn try_solve(
        &self,
        problem: &ProblemInstance,
        start_node: usize,
    ) -> Result<TspSolution, SolverError>;
}
