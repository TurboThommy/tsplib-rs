use tsplib_core::models::{Edge, TsplibInstance};

use crate::{PerfectMatchingAlgorithm, errors::MatcherError};

#[derive(Default)]
pub struct RecursiveMatching {}

impl RecursiveMatching {
    pub fn new() -> Self {
        Self {}
    }
}

const MAX_RECURSIVE_VERTICES: usize = 18;

impl PerfectMatchingAlgorithm for RecursiveMatching {
    fn try_compute(
        &self,
        odd_vertices: &[usize],
        problem: &TsplibInstance,
    ) -> Result<Vec<tsplib_core::models::Edge>, crate::errors::MatcherError> {
        tracing::debug!(
            odd_vertices = odd_vertices.len(),
            "Starting Recursive perfect matching computation"
        );

        if !odd_vertices.len().is_multiple_of(2) {
            return Err(MatcherError::OddVertexCountError(odd_vertices.len()));
        }

        if odd_vertices.len() > MAX_RECURSIVE_VERTICES {
            return Err(MatcherError::TooManyOddVertices(
                odd_vertices.len(),
                MAX_RECURSIVE_VERTICES,
            ));
        }

        let (best_cost, best_matching) = try_compute_recursive(odd_vertices, problem)?;

        tracing::debug!(
            matching_edges = best_matching.len(),
            matching_cost = best_cost,
            "Recursive perfect matching computation completed"
        );
        Ok(best_matching)
    }
}

fn try_compute_recursive(
    odd_vertices: &[usize],
    problem: &TsplibInstance,
) -> Result<(i64, Vec<Edge>), MatcherError> {
    if odd_vertices.is_empty() {
        return Ok((0, Vec::new()));
    }

    // safe unwrap since we checked for empty above
    let u = odd_vertices[0];

    // state variables to keep track of the best matching and its cost
    let mut best_matching = Vec::new();
    let mut best_cost = i64::MAX;

    // iterate over all possible matches
    for v_idx in 1..odd_vertices.len() {
        // get the vertex to match with u
        let v = odd_vertices[v_idx];

        // compute the remaining vertices after removing u and v
        let remaining = odd_vertices
            .iter()
            .enumerate()
            .filter_map(|(idx, &vertex)| {
                if idx == 0 || idx == v_idx {
                    None
                } else {
                    Some(vertex)
                }
            })
            .collect::<Vec<_>>();

        // recursively compute the best matching for the remaining vertices
        let (rest_cost, mut rest) = try_compute_recursive(&remaining, problem)?;

        // compute the cost of matching u and v, which is the distance between them
        // plus the cost of the best matching for the remaining vertices
        let uv_distance = problem.try_get_distance(u, v)?;
        let candidate_cost = i64::from(uv_distance) + rest_cost;

        // add the edge (u, v) to the matching for this candidate solution
        rest.push(Edge {
            u,
            v,
            weight: uv_distance,
        });

        // if this candidate solution is better than the best found so far, update the best matching and cost
        if candidate_cost < best_cost {
            best_cost = candidate_cost;
            best_matching = rest;
        }
    }

    Ok((best_cost, best_matching))
}
