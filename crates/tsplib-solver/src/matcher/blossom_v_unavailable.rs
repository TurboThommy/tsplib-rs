//! This module provides a fallback implementation of the Blossom V algorithm for perfect matching.
//! It is used when the actual Blossom V feature is not available.

use crate::{PerfectMatchingAlgorithm, errors::MatcherError};
use tsplib_core::models::{Edge, TsplibInstance};

pub struct BlossomVMatching {}

impl BlossomVMatching {
    pub fn new() -> Self {
        Self {}
    }
}

impl PerfectMatchingAlgorithm for BlossomVMatching {
    /// This method always returns an error indicating that the Blossom V algorithm is not available.
    fn try_compute(&self, _: &[usize], _: &TsplibInstance) -> Result<Vec<Edge>, MatcherError> {
        Err(MatcherError::BlossomVNotAvailable)
    }
}
