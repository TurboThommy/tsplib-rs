//! This module contains the `MatcherAlgorithmType` enum, which represents the different algorithms
//! that can be used for matching in the context of the Traveling Salesman Problem (TSP).
use serde::{Deserialize, Serialize};
use strum::EnumIter;

/// The `MatcherAlgorithm` enum represents the different algorithms that can be used for matching in the context of the Traveling Salesman Problem (TSP).
#[derive(Debug, Serialize, Deserialize, EnumIter)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum MatcherAlgorithm {
    #[default]
    Greedy,
    BlossomV,
}
