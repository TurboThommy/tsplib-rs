//! This module defines the `MstAlgorithmType` enum, which represents the different algorithms
//! that can be used to compute a minimum spanning tree (MST) in a graph.

use serde::{Deserialize, Serialize};
use strum::EnumIter;

/// An enumeration of the different algorithms that can be used to compute a minimum spanning tree (MST).
#[derive(Default, Debug, Serialize, Deserialize, EnumIter)]
#[serde(rename_all = "snake_case")]
pub enum MstAlgorithm {
    #[default]
    Kruskal,
    Prim,
    Boruvka,
}
