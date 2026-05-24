//! A simple greedy TSP solver that constructs a tour by always visiting the nearest unvisited node next. It also respects fixed edges if they are present in the problem instance.

use crate::{SolverOptions, TspSolver, errors::SolverError};
use std::collections::HashSet;
use tsplib_core::{
    context::ExecutionContext,
    enums::InstanceError,
    models::{TspSolution, TsplibInstance},
};

/// The Greedy algorithm is a simple heuristic for solving the TSP problem.
/// It constructs a tour by always visiting the nearest unvisited node next.
/// It also respects fixed edges if they are present in the problem instance.
pub struct Greedy {}

impl Greedy {
    /// Creates a new instance of the Greedy TSP solver.
    pub fn new() -> Self {
        Greedy {}
    }
}

impl Default for Greedy {
    fn default() -> Self {
        Self::new()
    }
}

impl TspSolver for Greedy {
    /// Solves the TSP problem using a greedy approach, starting from the specified node.
    /// It follows fixed edges if they exist and otherwise selects the nearest unvisited node.
    ///
    /// # Arguments
    /// * `problem` - A reference to the `ProblemInstance` representing the TSP problem to be solved.
    /// * `start_node` - The ID of the node from which the tour should start.
    /// * `ctx` - An `ExecutionContext` that allows for cancellation of the solving process.
    /// * `options` - A `SolverOptions` struct that can contain additional parameters for the solver (not used in this implementation).
    ///
    /// # Returns
    /// * `Result<TspSolution, SolverError>` - On success, returns a `TspSolution` containing the tour and its total cost.
    ///   On failure, returns a `SolverError` indicating the reason for the failure.
    fn try_solve_with_context(
        &self,
        problem: &TsplibInstance,
        start_node: usize,
        ctx: ExecutionContext,
        _: SolverOptions,
    ) -> Result<TspSolution, SolverError> {
        // check if the problem instance and start node are valid
        // and get the fixed edge map and targets for quick lookup
        let (fixed_edge_map, fixed_edge_targets) =
            self.try_check_problem_validity(problem, start_node)?;

        // create a tour starting from the start_node
        let mut tour: Vec<usize> = vec![start_node];
        // keep track of visited nodes to avoid cycles, HashSet is used for O(1) lookups
        let mut visited: HashSet<usize> = HashSet::from([start_node]);

        let mut current_node = start_node;
        let mut total_distance: i64 = 0;

        while visited.len() < problem.nodes.len() {
            // check for cancellation
            if ctx.is_cancelled() {
                return Err(SolverError::Cancelled);
            }

            // follow fixed edge if one exists for the current node
            if let Some(to) = fixed_edge_map.get(&current_node) {
                if visited.contains(to) {
                    return Err(SolverError::FixedEdgeToVisitedNode);
                }
                visited.insert(*to);
                tour.push(*to);
                total_distance += problem.try_get_distance(current_node, *to)? as i64;
                current_node = *to;
                continue;
            }

            // if there are no fixed edges, find the nearest unvisited node
            let next_node = problem
                .nodes
                .iter()
                .filter(|n| !visited.contains(&n.id) && !fixed_edge_targets.contains(&n.id))
                .map(|n| Ok((n, problem.try_get_distance(current_node, n.id)?)))
                .collect::<Result<Vec<_>, InstanceError>>()?
                .into_iter()
                .min_by_key(|(_, dist)| *dist)
                .map(|(n, _)| n);

            // if there is a next node, visit it; otherwise, return an error
            if let Some(next_node) = next_node {
                visited.insert(next_node.id);
                tour.push(next_node.id);
                total_distance += problem.try_get_distance(current_node, next_node.id)? as i64;
                current_node = next_node.id;
            } else {
                return Err(SolverError::NoUnvisitedNodes);
            }
        }

        // return to the starting node
        total_distance += problem.try_get_distance(current_node, start_node)? as i64;

        Ok(TspSolution {
            tour,
            cost: total_distance,
        })
    }
}
