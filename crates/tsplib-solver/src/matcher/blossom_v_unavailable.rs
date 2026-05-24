use tsplib_core::models::{Edge, TsplibInstance};

use crate::{PerfectMatchingAlgorithm, errors::MatcherError};

pub struct BlossomVMatching {}

impl PerfectMatchingAlgorithm for BlossomVMatching {
    fn new() -> Self {
        Self {}
    }

    fn try_compute(&self, _: &[usize], _: &TsplibInstance) -> Result<Vec<Edge>, MatcherError> {
        Err(MatcherError::BlossomVNotAvailable)
    }
}
