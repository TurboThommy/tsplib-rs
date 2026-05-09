//! This module defines the SolverError enum, which represents errors that can occur during TSP solving.

use thiserror::Error;
use tsplib_core::enums::InstanceError::{self, DistanceInvalidNodeId};

#[derive(Error, Debug)]
pub enum SolverError {
    #[error("Invalid start node provided.")]
    InvalidStartNode,
    #[error("Multiple fixed edges found for node {0}, which is not supported.")]
    MultipleFixedEdges(usize),
    #[error("No unvisited nodes found, but tour is not complete. This should never happen.")]
    NoUnvisitedNodes,
    #[error("Fixed edge leads to an already visited node.")]
    FixedEdgeToVisitedNode,
    #[error("Start node {0} is the target of a fixed edge, which is not supported.")]
    StartNodeIsFixedEdgeTarget(usize),
    #[error("Problem dimension exceeds the maximum allowed for this solver.")]
    DimensionExceeded,
    #[error(
        "Invalid dimension for Held-Karp solver. Maximum allowed is 64 due to bitmask limitations. Found {0}."
    )]
    HeldKarpInvalidDimension(usize),
    #[error("No solution found.")]
    NoSolution,
    #[error("Distance retrieval error: {0}")]
    DistanceRetrievalError(String),
}

impl From<InstanceError> for SolverError {
    fn from(value: InstanceError) -> Self {
        match value {
            DistanceInvalidNodeId(_, _, _) => {
                SolverError::DistanceRetrievalError(value.to_string())
            }
        }
    }
}
