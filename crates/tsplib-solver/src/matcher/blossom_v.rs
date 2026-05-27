//! This module implements the Blossom V algorithm for finding a minimum weight perfect matching in a graph.

use crate::{PerfectMatchingAlgorithm, errors::MatcherError};
use blossom_v::try_solve_min_weight_perfect_matching;
use tsplib_core::models::{Edge, TsplibInstance};

pub struct BlossomVMatching {}

impl BlossomVMatching {
    pub fn new() -> Self {
        Self {}
    }
}

impl PerfectMatchingAlgorithm for BlossomVMatching {
    /// Computes a minimum weight perfect matching for the given odd vertices and TSP instance using the Blossom V algorithm.
    ///
    /// # Arguments
    /// * `odd_vertices` - A slice of vertex indices that have odd degree in the current solution.
    /// * `problem` - A reference to the TSP instance containing the distance matrix and other relevant information.
    ///
    /// # Returns
    /// * `Result<Vec<Edge>, MatcherError>` - A result containing a vector of edges in the minimum weight perfect matching,
    ///   or an error if the computation fails.
    fn try_compute(
        &self,
        odd_vertices: &[usize],
        problem: &TsplibInstance,
    ) -> Result<Vec<Edge>, MatcherError> {
        tracing::debug!(
            odd_vertices = odd_vertices.len(),
            edge_candidates = odd_vertices.len() * (odd_vertices.len() - 1) / 2,
            "Starting Blossom V perfect matching"
        );

        // Generate the complete graph of odd vertices with edge weights corresponding to the distances in the TSP instance.
        let mut edges = Vec::new();

        // Create edges for the complete graph of odd vertices
        for i in 0..odd_vertices.len() {
            // Only consider pairs (i, j) where j > i to avoid duplicates
            for j in (i + 1)..odd_vertices.len() {
                // Get the original vertex indices from the odd_vertices slice
                let u = odd_vertices[i];
                let v = odd_vertices[j];

                // Get the weight (distance) between vertices u and v from the TSP instance
                let weight = problem.try_get_distance(u, v)?;

                // Add the edge (i, j) with the corresponding weight to the edges vector
                edges.push((i, j, weight));
            }
        }

        // Use the Blossom V algorithm to find the minimum weight perfect matching in the complete graph of odd vertices.
        let matching = try_solve_min_weight_perfect_matching(odd_vertices.len(), &edges)?;

        // Convert the matching from indices in the complete graph back to the original vertex indices and create Edge instances.
        let result = matching
            .into_iter()
            .map(|(i, j)| {
                // Get the original vertex indices from the odd_vertices slice
                let u = odd_vertices[i];
                let v = odd_vertices[j];

                // Get the weight (distance) between vertices u and v from the TSP instance
                let weight = problem.try_get_distance(u, v)?;

                // Create an Edge instance for the matched pair of vertices
                Ok(Edge { u, v, weight })
            })
            .collect::<Result<Vec<_>, MatcherError>>()?;

        tracing::debug!(
            matching_edges = result.len(),
            "Blossom V perfect matching completed"
        );

        Ok(result)
    }
}
