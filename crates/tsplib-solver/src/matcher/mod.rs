//! Matching algorithms for the TSP.

/// This module defines the `GreedyMatching` struct, which implements a greedy algorithm for finding a matching in a graph.
mod greedy;
pub use greedy::GreedyMatching;

/// This module defines the `RecursiveMatching` struct, which implements a recursive algorithm for finding an exact minimum weight perfect matching in a graph.
mod recursive;
pub use recursive::RecursiveMatching;

mod edmonds;
pub use edmonds::WeightedEdmondsMatching;

/// This module defines the `BlossomVMatching` struct, which implements the Blossom V algorithm for finding a minimum weight perfect matching in a graph.
/// The implementation is only available if the `blossom-v` feature is enabled.
#[cfg(feature = "blossom-v")]
mod blossom_v;
#[cfg(feature = "blossom-v")]
pub use blossom_v::BlossomVMatching;

/// This module defines a placeholder `BlossomVMatching` struct that is used when the `blossom-v` feature is not enabled.
/// It provides a compile-time error message to inform the user that the Blossom V algorithm is not available without the feature.
#[cfg(not(feature = "blossom-v"))]
mod blossom_v_unavailable;
#[cfg(not(feature = "blossom-v"))]
pub use blossom_v_unavailable::BlossomVMatching;
