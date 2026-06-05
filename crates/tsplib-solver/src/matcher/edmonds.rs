//! This module implements the Edmonds' Blossom algorithm for finding a minimum weight perfect matching in a graph.
use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};

use tsplib_core::models::{Edge, Graph, TsplibInstance};

use crate::{PerfectMatchingAlgorithm, errors::MatcherError};

#[derive(Default)]
pub struct WeightedEdmondsMatching {}

impl WeightedEdmondsMatching {
    pub fn new() -> Self {
        Self {}
    }
}

impl PerfectMatchingAlgorithm for WeightedEdmondsMatching {
    fn try_compute(
        &self,
        odd_vertices: &[usize],
        problem: &tsplib_core::models::TsplibInstance,
    ) -> Result<Vec<Edge>, MatcherError> {
        if !odd_vertices.len().is_multiple_of(2) {
            return Err(MatcherError::OddVertexCountError(odd_vertices.len()));
        }

        let mut odd_vertices = odd_vertices.to_vec();
        odd_vertices.sort_unstable();

        let _graph = try_build_complete_graph_for_vertices(&odd_vertices, problem)?;

        todo!("WeightedEdmondsMatching is not implemented yet")
    }
}

/// Helper function to create a consistent key for an edge between two vertices, regardless of their order.
///
/// # Arguments
/// * `u` - The index of the first vertex.
/// * `v` - The index of the second vertex.
///
/// # Returns
/// * `(usize, usize)` - A tuple representing the edge between the two vertices, with the smaller index first to ensure consistency.
fn edge_key(u: usize, v: usize) -> (usize, usize) {
    if u < v { (u, v) } else { (v, u) }
}

/// Builds a complete graph for the given vertices based on the distances in the TSP instance.
///
/// # Arguments
/// * `vertices` - A slice of vertex indices for which to build the complete graph.
/// * `problem` - The TSP instance containing the nodes and distance information.
///
/// # Returns
/// * `Result<Graph, MatcherError>` - A result containing the complete graph or
///   an error if any vertex is not found or if distance retrieval fails.
fn try_build_complete_graph_for_vertices(
    vertices: &[usize],
    problem: &TsplibInstance,
) -> Result<Graph, MatcherError> {
    let nodes = vertices
        .iter()
        .map(|&id| {
            problem
                .nodes
                .iter()
                .find(|node| node.id == id)
                .copied()
                .ok_or(MatcherError::NoMatchingCandidate(id))
        })
        .collect::<Result<Vec<_>, _>>()?;

    let mut edges = Vec::new();

    for i in 0..nodes.len() {
        for j in (i + 1)..vertices.len() {
            let u = vertices[i];
            let v = vertices[j];

            let weight = problem.try_get_distance(u, v)?;

            edges.push(Edge { u, v, weight });
        }
    }

    Ok(Graph { nodes, edges })
}

#[cfg(test)]
mod oracle_tests;
#[cfg(test)]
mod tests;
