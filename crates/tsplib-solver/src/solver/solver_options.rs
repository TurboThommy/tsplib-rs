//! This module provides a struct for configuring solver options.

use serde::Deserialize;

use crate::enums::{MatcherAlgorithm, MstAlgorithm};

#[derive(Default, Deserialize)]
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
