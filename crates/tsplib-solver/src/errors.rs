//! This module defines the SolverError enum, which represents errors that can occur during TSP solving.

use thiserror::Error;

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
}
