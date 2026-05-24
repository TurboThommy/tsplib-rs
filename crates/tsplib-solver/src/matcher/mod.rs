mod greedy;
pub use greedy::GreedyMatching;

#[cfg(feature = "blossom-v")]
mod blossom_v;
#[cfg(feature = "blossom-v")]
pub use blossom_v::BlossomVMatching;

#[cfg(not(feature = "blossom-v"))]
mod blossom_v_unavailable;
#[cfg(not(feature = "blossom-v"))]
pub use blossom_v_unavailable::BlossomVMatching;
