//! This crate provides a trait for solving the Traveling Salesman Problem (TSP) using various algorithms.
pub mod errors;
pub mod greedy;
pub mod held_karp;

use std::collections::{HashMap, HashSet};

pub use greedy::Greedy;
pub use held_karp::HeldKarp;

use errors::SolverError;
use tsplib_core::models::{ProblemInstance, TspSolution};

pub trait TspSolver {
    /// Solves the TSP problem for the given problem instance and starting node.
    /// The implementation of this method should return a `TspSolution` containing a tour and its total cost,
    /// or a `SolverError` if the problem cannot be solved.
    ///
    /// # Arguments
    /// * `problem` - A reference to the `ProblemInstance` representing the TSP problem to be solved.
    /// * `start_node` - The ID of the node from which the tour should start.
    ///
    /// # Returns
    /// * `Result<TspSolution, SolverError>` - On success, returns a `TspSolution` containing the optimal tour and its total cost.
    ///   On failure, returns a `SolverError` indicating the reason for the failure
    ///   (e.g., invalid start node, dimension exceeded, no solution found, etc.).
    fn try_solve(
        &self,
        problem: &ProblemInstance,
        start_node: usize,
    ) -> Result<TspSolution, SolverError>;

    /// Checks the validity of the problem instance and the starting node for the TSP solver.
    ///
    /// # Arguments
    /// * `problem` - A reference to the `ProblemInstance` representing the TSP problem to be solved.
    /// * `start_node` - The ID of the node from which the tour should start.
    ///
    /// # Returns
    /// * `Result<(HashMap<usize, usize>, HashSet<usize>), SolverError>`
    ///   * `HashMap<usize, usize>`  - A `HashMap` mapping each node ID to its fixed edge target (if it has one).
    ///   * `HashSet<usize>` - A `HashSet` containing the IDs of all nodes that are targets of fixed edges.
    ///   * `SolverError` - Error indicating the reason for the failure
    ///     (e.g., invalid start node, start node is a fixed edge target, multiple fixed edges from the same node, etc.).
    fn try_check_problem_validity(
        &self,
        problem: &ProblemInstance,
        start_node: usize,
    ) -> Result<(HashMap<usize, usize>, HashSet<usize>), SolverError> {
        // check if start_node is valid
        if !problem.nodes.iter().any(|n| n.id == start_node) {
            return Err(SolverError::InvalidStartNode);
        }

        // collect all fixed edges and their targets for quick lookup
        let fixed_edges = problem.fixed_edges.iter().flatten().collect::<Vec<_>>();
        let fixed_edge_targets = fixed_edges
            .iter()
            .map(|(_, to)| *to)
            .collect::<HashSet<usize>>();

        let fixed_edge_map = fixed_edges
            .iter()
            .map(|(from, to)| (*from, *to))
            .collect::<HashMap<usize, usize>>();

        // check if start_node is target of a fixed edge
        if fixed_edge_targets.contains(&start_node) {
            return Err(SolverError::StartNodeIsFixedEdgeTarget(start_node));
        }

        // check if any node has multiple fixed edges
        let max_fixed_edges = fixed_edges
            .iter()
            .fold(HashMap::new(), |mut acc, (from, _)| {
                *acc.entry(*from).or_insert(0) += 1;
                acc
            })
            .into_iter()
            .find(|(_, count)| *count > 1);

        if let Some((node_id, _)) = max_fixed_edges {
            return Err(SolverError::MultipleFixedEdges(node_id));
        }

        Ok((fixed_edge_map, fixed_edge_targets))
    }
}
