use tsplib_core::models::{Edge, TsplibInstance};

use crate::{PerfectMatchingAlgorithm, errors::MatcherError};

pub struct BlossomVMatching {}

impl BlossomVMatching {
    pub fn new() -> Self {
        Self {}
    }
}

impl PerfectMatchingAlgorithm for BlossomVMatching {
    fn try_compute(&self, _: &[usize], _: &TsplibInstance) -> Result<Vec<Edge>, MatcherError> {
        Err(MatcherError::BlossomVNotAvailable)
    }
}
