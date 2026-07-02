//! This module contains a greedy algorithm for computing a perfect matching on a set of odd-degree vertices in a graph, which is used as part of the Christofides algorithm for solving the TSP.

use crate::{PerfectMatchingAlgorithm, errors::MatcherError};
use tsplib_core::models::{Edge, TsplibInstance};

#[derive(Default)]
pub struct GreedyMatching {}

impl GreedyMatching {
    pub fn new() -> Self {
        Self {}
    }
}

impl PerfectMatchingAlgorithm for GreedyMatching {
    /// Computes a perfect matching on the given set of odd-degree vertices using a greedy approach.
    ///
    /// # Arguments
    /// * `odd_vertices` - A slice of vertex indices that have odd degree in the graph.
    /// * `problem` - The TSP instance containing the adjacency matrix for edge weights.
    ///
    /// # Returns
    /// * `Result<Vec<Edge>, MatcherError>` - A vector of edges representing the perfect matching, or an error if the input is invalid.
    fn try_compute(
        &self,
        odd_vertices: &[usize],
        problem: &TsplibInstance,
    ) -> Result<Vec<tsplib_core::models::Edge>, crate::errors::MatcherError> {
        tracing::debug!(
            odd_vertices = odd_vertices.len(),
            "Starting Greedy perfect matching computation"
        );

        // Ensure that the number of odd vertices is even, as a perfect matching is only possible in this case.
        if !odd_vertices.len().is_multiple_of(2) {
            return Err(MatcherError::OddVertexCountError(odd_vertices.len()));
        }

        // Create a mutable list of unmatched vertices and an empty list to store the resulting matching.
        let mut unmatched = odd_vertices.to_vec();
        let mut matching = Vec::new();

        // While there are unmatched vertices, repeatedly find the closest pair of vertices and add the corresponding edge to the matching.
        while let Some(u) = unmatched.pop() {
            // Find the unmatched vertex that is closest to u based on the adjacency matrix.
            let (best_index, _) = unmatched
                .iter()
                .enumerate()
                // .min_by_key(|(_, v)| problem.adjacency_matrix[u - 1][*v - 1])
                .min_by_key(|(_, v)| problem.try_get_distance(u, **v).unwrap_or(i32::MAX))
                .ok_or(MatcherError::NoMatchingCandidate(u))?;

            // Remove the best match from the unmatched list and add the edge to the matching.
            let v = unmatched.remove(best_index);
            // Get the weight of the edge between u and v from the adjacency matrix.
            // let weight = problem.adjacency_matrix[u - 1][v - 1];
            let weight = problem.try_get_distance(u, v).unwrap_or(i32::MAX);

            // Add the edge (u, v) with the corresponding weight to the matching.
            matching.push(Edge { u, v, weight });
        }

        tracing::debug!(
            matching_edges = matching.len(),
            "Greedy perfect matching completed"
        );

        Ok(matching)
    }
}
