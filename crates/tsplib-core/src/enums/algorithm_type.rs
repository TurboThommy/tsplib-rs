// ! This module contains the definition of the `AlgorithmType` enum, which represents the different types of algorithms that can be used to solve the TSP problem.
use serde::{Deserialize, Serialize};
use strum::EnumIter;

#[derive(Debug, Serialize, Deserialize, EnumIter)]
#[serde(rename_all = "snake_case")]
pub enum AlgorithmType {
    HeldKarp,
    Greedy,
}
