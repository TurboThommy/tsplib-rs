//! This module contains the `MatcherAlgorithmType` enum, which represents the different algorithms
//! that can be used for matching in the context of the Traveling Salesman Problem (TSP).
use serde::{Deserialize, Serialize};
use strum::EnumIter;

#[derive(Debug, Serialize, Deserialize, EnumIter)]
#[serde(rename_all = "snake_case")]
pub enum MatcherAlgorithm {
    Greedy,
    BlossomV,
}
