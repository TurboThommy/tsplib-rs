//! This module defines the `MstAlgorithmType` enum, which represents the different algorithms
//! that can be used to compute a minimum spanning tree (MST) in a graph.

use serde::{Deserialize, Serialize};
use strum::EnumIter;

#[derive(Debug, Serialize, Deserialize, EnumIter)]
pub enum MstAlgorithm {
    Kruskal,
    Prim,
    Boruvka,
}
