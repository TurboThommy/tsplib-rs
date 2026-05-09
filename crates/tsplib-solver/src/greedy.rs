//! A simple greedy TSP solver that constructs a tour by always visiting the nearest unvisited node next. It also respects fixed edges if they are present in the problem instance.

use std::collections::{HashMap, HashSet};

use tsplib_core::{
    enums::InstanceError,
    models::{ProblemInstance, TspSolution},
};

use crate::{TspSolver, errors::SolverError};

/// The Greedy algorithm is a simple heuristic for solving the TSP problem.
/// It constructs a tour by always visiting the nearest unvisited node next.
/// It also respects fixed edges if they are present in the problem instance.
pub struct Greedy {}

impl TspSolver for Greedy {
    /// Solves the TSP problem using a greedy approach, starting from the specified node.
    /// It follows fixed edges if they exist and otherwise selects the nearest unvisited node.
    ///
    /// # Arguments
    /// * `problem` - A reference to the `ProblemInstance` representing the TSP problem to be solved.
    /// * `start_node` - The ID of the node from which the tour should start.
    ///
    /// # Returns
    /// * `Result<TspSolution, SolverError>` - On success, returns a `TspSolution` containing the tour and its total cost.
    ///   On failure, returns a `SolverError` indicating the reason for the failure.
    fn try_solve(
        &self,
        problem: &ProblemInstance,
        start_node: usize,
    ) -> Result<TspSolution, SolverError> {
        // check if start_node is valid
        if !problem.nodes.iter().any(|n| n.id == start_node) {
            return Err(SolverError::InvalidStartNode);
        }

        // collect all fixed edges and their targets for quick lookup
        let fixed_edges = problem.fixed_edges.iter().flatten().collect::<Vec<_>>();
        let fixed_edges_targets = fixed_edges
            .iter()
            .map(|(_, to)| *to)
            .collect::<HashSet<usize>>();

        // check if start_node is target of a fixed edge
        if fixed_edges_targets.contains(&start_node) {
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

        // create a tour starting from the start_node
        let mut tour: Vec<usize> = vec![start_node];
        // keep track of visited nodes to avoid cycles, HashSet is used for O(1) lookups
        let mut visited: HashSet<usize> = HashSet::from([start_node]);

        let mut current_node = start_node;
        let mut total_distance: i64 = 0;

        while visited.len() < problem.nodes.len() {
            // follow fixed edge if one exists for the current node
            if let Some((_, to)) = fixed_edges.iter().find(|(from, _)| *from == current_node) {
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
                .filter(|n| !visited.contains(&n.id) && !fixed_edges_targets.contains(&n.id))
                .map(|n| Ok((n, problem.try_get_distance(current_node, n.id)?)))
                .collect::<Result<Vec<_>, InstanceError>>()?
                .into_iter()
                .min_by_key(|(_, dist)| *dist)
                .map(|(n, _)| n);

            if let Some(next_node) = next_node {
                visited.insert(next_node.id);
                tour.push(next_node.id);
                total_distance += problem.try_get_distance(current_node, next_node.id)? as i64;
                current_node = next_node.id;
            } else {
                return Err(SolverError::NoUnvisitedNodes);
            }
        }
        total_distance += problem.try_get_distance(current_node, start_node)? as i64; // return to the starting node

        Ok(TspSolution {
            tour,
            cost: total_distance,
        })
    }
}
