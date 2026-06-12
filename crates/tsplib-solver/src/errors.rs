//! This module defines the SolverError enum, which represents errors that can occur during TSP solving.

#[cfg(feature = "blossom-v")]
use blossom_v::BlossomVError;

use thiserror::Error;
use tsplib_core::enums::{
    GraphError,
    InstanceError::{self},
    MstComputationError,
};

/// Errors that can occur during TSP solving.
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

/// Errors that can occur during the perfect matching step of the Christofides algorithm.
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
    #[error(
        "Blossom V algorithm is not available. Ensure that the blossom-v feature is enabled and the library is properly set up."
    )]
    BlossomVNotAvailable,
    #[error("Too many odd vertices ({0}) for matcher. Maximum allowed is {1}.")]
    TooManyOddVertices(usize, usize),
    #[error("Invalid augmenting path found during matching.")]
    InvalidAugmentingPath,
    #[error("Path is not connected to root during reconstruction.")]
    PathReconstructionError,
    #[error("Expected matched node {0} to have a mate, but it was missing.")]
    MissingMate(usize),
    #[error(
        "Node {0} is not connected to its least common ancestor {1} during path reconstruction."
    )]
    NodeNotConnectedToLca(usize, usize),

    #[error("Node {0} is not in the blossom during path reconstruction.")]
    NodeNotInBlossom(usize),
    #[error("No alternating blossom path found between nodes {0} and {1}.")]
    NoAlternatingBlossomPath(usize, usize),
    #[error("Shrunk node {0} does not have a corresponding original node mapping.")]
    ShrunkNodeNotMapped(usize),
    #[error(
        "Blossom node is at the boundary of the shrunk path and cannot be expanded. Position: {0}"
    )]
    BlossomNodeAtPathBoundary(usize),
    #[error("No original edge found from external node {0} into blossom.")]
    NoEdgeIntoBlossom(usize),
    #[error("Invalid node index {0} for graph with {1} nodes.")]
    InvalidNodeIndex(usize, usize),
    #[error("Edge ({0}, {1}) does not exist in graph.")]
    MissingEdge(usize, usize),
    #[error("No solution found for the matching problem.")]
    NoSolution,
    #[error("Node {0} was left unmatched by the final lift (no perfect matching produced).")]
    NodeUnmatched(usize),
    #[error(
        "The final matching still referenced a pseudonode slot, the lift was incomplete. (node: {0}, mate: {1})"
    )]
    MateNotLifted(usize, usize),
    #[error(
        "An internal invariant of the matcher was violated. This should never happen. Details: {0}"
    )]
    Internal(&'static str),
    #[error("Blossom expansion is not implemented in this matcher.")]
    BlossomExpansionNotImplemented,
}

#[derive(Error, Debug, PartialEq)]
pub enum SimplexError {
    #[error("The linear program is unbounded.")]
    Unbounded,
}

impl From<InstanceError> for SolverError {
    fn from(value: InstanceError) -> Self {
        SolverError::DistanceRetrievalError(value.to_string())
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
        MatcherError::BlossomVError(value.to_string())
    }
}
