//! This module provides a struct for configuring solver options.

use crate::enums::{MatcherAlgorithm, MstAlgorithm};
use serde::Deserialize;

/// The `SolverOptions` struct holds optional configuration settings for the TSP solver.
#[derive(Debug, Default, Deserialize)]
pub struct SolverOptions {
    /// Optional MST algorithm to use in the Christofides algorithm.
    pub mst_algorithm: Option<MstAlgorithm>,
    /// Optional matching algorithm to use in the Christofides algorithm.
    pub matcher_algorithm: Option<MatcherAlgorithm>,
}

impl SolverOptions {
    pub fn new() -> Self {
        Self {
            mst_algorithm: None,
            matcher_algorithm: None,
        }
    }
}
