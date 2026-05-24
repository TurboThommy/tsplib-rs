//! This module defines the SolverError enum, which represents errors that can occur during TSP solving.

#[cfg(feature = "blossom-v")]
use blossom_v::BlossomVError;

use thiserror::Error;
use tsplib_core::enums::{
    GraphError,
    InstanceError::{self, DistanceInvalidNodeId},
    MstComputationError,
};

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
    #[error("Invalid parent table entry during tour reconstruction.")]
    InvalidParentTable,
    #[error("Solver was cancelled.")]
    Cancelled,
    #[error("Fixed edges are not supported by this solver.")]
    FixedEdgesNotSupported,
    #[error("Error during minimum spanning tree computation: {0}")]
    GetMstError(String),
    #[error("Error finding perfect matching: {0}")]
    MatcherError(String),
    #[error("Error finding Eulerian circuit: {0}")]
    EulerianCircuitError(String),
}

#[derive(Error, Debug)]
pub enum MatcherError {
    #[error("Odd vertex count should be even. Found {0} odd vertices.")]
    OddVertexCountError(usize),
    #[error("No matching candidate found for vertex {0}.")]
    NoMatchingCandidate(usize),
    #[error("Error from Blossom V algorithm: {0}")]
    BlossomVError(String),
    #[error("Matcher failed due to a distance retrieval error: {0}")]
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

impl From<MstComputationError> for SolverError {
    fn from(value: MstComputationError) -> Self {
        SolverError::GetMstError(value.to_string())
    }
}

impl From<MatcherError> for SolverError {
    fn from(value: MatcherError) -> Self {
        SolverError::MatcherError(value.to_string())
    }
}

impl From<GraphError> for SolverError {
    fn from(value: GraphError) -> Self {
        match value {
            GraphError::EulerianCircuitOddDegreeError
            | GraphError::EulerianCircuitDisconnectedGraphError
            | GraphError::EulerianCircuitEmptyGraphError => {
                SolverError::EulerianCircuitError(value.to_string())
            }
        }
    }
}

#[cfg(feature = "blossom-v")]
impl From<BlossomVError> for MatcherError {
    fn from(value: BlossomVError) -> Self {
        MatcherError::BlossomVError(value.to_string())
    }
}

impl From<InstanceError> for MatcherError {
    fn from(value: InstanceError) -> Self {
        match value {
            DistanceInvalidNodeId(_, _, _) => MatcherError::BlossomVError(value.to_string()),
        }
    }
}
