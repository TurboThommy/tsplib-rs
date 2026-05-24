use blossom_v::try_solve_min_weight_perfect_matching;
use tsplib_core::models::{Edge, TsplibInstance};

use crate::{PerfectMatchingAlgorithm, errors::MatcherError};

pub struct BlossomVMatching {}

impl PerfectMatchingAlgorithm for BlossomVMatching {
    fn new() -> Self {
        Self {}
    }

    fn try_compute(
        &self,
        odd_vertices: &[usize],
        problem: &TsplibInstance,
    ) -> Result<Vec<Edge>, MatcherError> {
        let mut edges = Vec::new();

        for i in 0..odd_vertices.len() {
            for j in (i + 1)..odd_vertices.len() {
                let u = odd_vertices[i];
                let v = odd_vertices[j];

                let weight = problem.try_get_distance(u, v)?;

                edges.push((i, j, weight));
            }
        }

        let matching = try_solve_min_weight_perfect_matching(odd_vertices.len(), &edges)?;

        let result = matching
            .into_iter()
            .map(|(i, j)| {
                let u = odd_vertices[i];
                let v = odd_vertices[j];

                let weight = problem.try_get_distance(u, v)?;

                Ok(Edge { u, v, weight })
            })
            .collect::<Result<Vec<_>, MatcherError>>()?;

        Ok(result)
    }
}
