//! Defines the ProblemType enum, which specifies the type of combinatorial optimization problem being defined in the instance file.
use std::fmt;

use serde::Serialize;

/// ProblemType specifies the type of combinatorial optimization problem being defined in the instance file.
#[derive(Debug, Clone, Serialize)]
pub enum ProblemType {
    /// Symmetric TSP
    /// distance between i and j is the same as between j and i
    TSP,

    /// Asymmetric TSP
    /// distance from i to j may differ from distance from j to i
    ATSP,

    /// Sequential Ordering Problem
    /// ATSP with precedence constraints, where certain vertices must be visited before others
    SOP,

    /// Hamilton Cycle Problem
    /// Test if the graph contains a Hamilton cycle (a cycle that visits each vertex exactly once)
    HCP,

    /// Capacitated Vehicle Routing Problem
    /// TSP with multiple vehicles and capacity constraints
    CVRP,

    /// Collection of tours
    /// TBD
    TOUR,
}

impl fmt::Display for ProblemType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            ProblemType::TSP => "TSP",
            ProblemType::ATSP => "ATSP",
            ProblemType::SOP => "SOP",
            ProblemType::HCP => "HCP",
            ProblemType::CVRP => "CVRP",
            ProblemType::TOUR => "TOUR",
        };
        write!(f, "{}", s)
    }
}
