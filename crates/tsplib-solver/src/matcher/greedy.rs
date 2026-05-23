//! This module contains a greedy algorithm for computing a perfect matching on a set of odd-degree vertices in a graph, which is used as part of the Christofides algorithm for solving the TSP.

use tsplib_core::models::Edge;

use crate::{PerfectMatchingAlgorithm, errors::MatcherError};

pub struct GreedyMatching {}

impl PerfectMatchingAlgorithm for GreedyMatching {
    fn new() -> Self {
        GreedyMatching {}
    }

    fn try_compute(
        &self,
        odd_vertices: &[usize],
        problem: &tsplib_core::models::TsplibInstance,
    ) -> Result<Vec<tsplib_core::models::Edge>, crate::errors::MatcherError> {
        if !odd_vertices.len().is_multiple_of(2) {
            return Err(MatcherError::OddVertexCountError(odd_vertices.len()));
        }

        let mut unmatched = odd_vertices.to_vec();
        let mut matching = Vec::new();

        while let Some(u) = unmatched.pop() {
            let (best_index, _) = unmatched
                .iter()
                .enumerate()
                .min_by_key(|(_, v)| problem.adjacency_matrix[u - 1][*v - 1])
                .ok_or(MatcherError::NoMatchingCandidate(u))?;

            let v = unmatched.remove(best_index);
            let weight = problem.adjacency_matrix[u - 1][v - 1];

            matching.push(Edge { u, v, weight });
        }

        Ok(matching)
    }
}
