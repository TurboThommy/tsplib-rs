//! This module defines data structures for representing graphs, including nodes, edges, and the graph itself.
//! It also includes helper functions to calculate distances between nodes.

use std::collections::HashMap;

use crate::{distances::*, enums::GraphError};
use serde::Serialize;

/// A struct representing the node of a graph.
#[derive(Debug, Copy, Clone, Serialize)]
pub struct Node {
    /// The unique identifier of the node.
    pub id: usize,
    /// The x-coordinate of the node.
    pub x: f64,
    /// The y-coordinate of the node.
    pub y: f64,
    /// The optional z-coordinate of the node, which may be used for 3D coordinates.
    pub z: Option<f64>,
}

/// A struct representing an edge between two nodes in a graph, with its weight.
#[derive(Debug, Copy, Clone)]
pub struct Edge {
    /// The ID of the first node.
    pub u: usize,
    /// The ID of the second node.
    pub v: usize,
    /// The weight of the edge.
    pub weight: i32,
}

/// A struct representing a graph, consisting of a collection of nodes and edges.
#[derive(Debug, Clone)]
pub struct Graph {
    /// The nodes in the graph.
    pub nodes: Vec<Node>,
    /// The edges in the graph.
    pub edges: Vec<Edge>,
}

impl Node {
    /// Calculates the Euclidean distance between this node and another node in 2D space.
    ///
    /// # Arguments
    /// * `other` - Reference to another `Node` to which the distance will be calculated.
    ///
    /// # Returns
    /// * `i32` - The calculated distance rounded to the nearest integer.
    ///   Returns 0 if the nodes are the same (i.e., have the same ID).
    pub(super) fn _distance_euc_2d(&self, other: &Node) -> i32 {
        distance_euc_2d((self.id, self.x, self.y), (other.id, other.x, other.y))
    }

    /// Calculates the Euclidean distance between this node and another node in 3D space.
    ///
    /// # Arguments
    /// * `other` - Reference to another `Node` to which the distance will be calculated.
    ///
    /// # Returns
    /// * `i32` - The calculated distance rounded to the nearest integer.
    ///   Returns 0 if the nodes are the same (i.e., have the same ID).
    ///   If either node does not have a z-coordinate, it is treated as 0 for the distance calculation.
    pub(super) fn _distance_euc_3d(&self, other: &Node) -> i32 {
        _distance_euc_3d(
            (self.id, self.x, self.y, self.z),
            (other.id, other.x, other.y, other.z),
        )
    }

    /// Calculates the Manhattan distance between this node and another node in 2D space.
    ///
    /// # Arguments
    /// * `other` - Reference to another `Node` to which the distance will be calculated.
    ///
    /// # Returns
    /// * `i32` - The calculated distance rounded to the nearest integer.
    ///   Returns 0 if the nodes are the same (i.e., have the same ID).
    pub(super) fn _distance_man_2d(&self, other: &Node) -> i32 {
        distance_man_2d((self.id, self.x, self.y), (other.id, other.x, other.y))
    }

    /// Calculates the Manhattan distance between this node and another node in 3D space.
    ///
    /// # Arguments
    /// * `other` - Reference to another `Node` to which the distance will be calculated.
    ///
    /// # Returns
    /// * `i32` - The calculated distance rounded to the nearest integer.
    ///   Returns 0 if the nodes are the same (i.e., have the same ID).
    ///   If either node does not have a z-coordinate, it is treated as 0 for the distance calculation.
    pub(super) fn _distance_man_3d(&self, other: &Node) -> i32 {
        _distance_man_3d(
            (self.id, self.x, self.y, self.z),
            (other.id, other.x, other.y, other.z),
        )
    }

    /// Calculates the maximum distance between this node and another node in 2D space.
    ///
    /// # Arguments
    /// * `other` - Reference to another `Node` to which the distance will be calculated.
    ///
    /// # Returns
    /// * `i32` - The calculated distance rounded to the nearest integer.
    ///   Returns 0 if the nodes are the same (i.e., have the same ID).
    pub(super) fn _distance_max_2d(&self, other: &Node) -> i32 {
        distance_max_2d((self.id, self.x, self.y), (other.id, other.x, other.y))
    }

    /// Calculates the maximum distance between this node and another node in 3D space.
    ///
    /// # Arguments
    /// * `other` - Reference to another `Node` to which the distance will be calculated.
    ///
    /// # Returns
    /// * `i32` - The calculated distance rounded to the nearest integer.
    ///   Returns 0 if the nodes are the same (i.e., have the same ID).
    ///   If either node does not have a z-coordinate, it is treated as 0 for the distance calculation.
    pub(super) fn _distance_max_3d(&self, other: &Node) -> i32 {
        _distance_max_3d(
            (self.id, self.x, self.y, self.z),
            (other.id, other.x, other.y, other.z),
        )
    }

    /// Calculates the Euclidean distance between this node and another node in 2D space, rounded up to the nearest integer.
    ///
    /// # Arguments
    /// * `other` - Reference to another `Node` to which the distance will be calculated.
    ///
    /// # Returns
    /// * `i32` - The calculated distance rounded up to the nearest integer.
    ///   Returns 0 if the nodes are the same (i.e., have the same ID).
    pub(super) fn _distance_ceil_2d(&self, other: &Node) -> i32 {
        distance_ceil_2d((self.id, self.x, self.y), (other.id, other.x, other.y))
    }

    /// Calculates the pseudo-Euclidean distance between this node and another node as
    /// defined in the TSPLIB specification for ATT coordinates.
    ///
    /// # Arguments
    /// * `other` - Reference to another `Node` to which the distance will be calculated.
    ///
    /// # Returns
    /// * `i32` - The calculated distance rounded to the nearest integer.
    ///   Returns 0 if the nodes are the same (i.e., have the same ID).
    pub(super) fn _distance_att(&self, other: &Node) -> i32 {
        distance_att((self.id, self.x, self.y), (other.id, other.x, other.y))
    }

    /// Calculate the geographical distance between this node and another node
    /// using the formula provided in the TSPLIB specification for GEO coordinates.
    ///
    /// # Arguments
    /// * `other` - Reference to another `Node` to which the distance will be calculated.
    ///
    /// # Returns
    /// * `i32` - The calculated distance rounded to the nearest integer.
    ///   Returns 0 if the nodes are the same (i.e., have the same ID).
    pub(super) fn _distance_geo(&self, other: &Node) -> i32 {
        distance_geo((self.id, self.x, self.y), (other.id, other.x, other.y))
    }
}

impl Graph {
    /// Calculates the degree of each node in the graph and returns a `HashMap` mapping node IDs to their degrees.
    ///
    /// # Returns
    /// * `HashMap<usize, usize>` - A `HashMap` where the keys are node IDs and the values are the corresponding degrees of those nodes in the graph.
    pub fn get_degrees(&self) -> HashMap<usize, usize> {
        let mut degrees = HashMap::new();

        self.edges.iter().for_each(|edge| {
            *degrees.entry(edge.u).or_insert(0) += 1;
            *degrees.entry(edge.v).or_insert(0) += 1;
        });

        degrees
    }

    /// Tries to find an Eulerian circuit in the graph using Hierholzer's algorithm.
    ///
    /// # Returns
    /// * `Result<Vec<usize>, GraphError>` - On success, returns a `Vec<usize>` containing the sequence of node IDs in the Eulerian circuit.
    ///   On failure, returns a `GraphError` indicating the reason for the failure (e.g., odd degree nodes, disconnected graph, empty graph, etc.).
    pub fn try_get_eulerian_circuit(&self) -> Result<Vec<usize>, GraphError> {
        tracing::debug!(
            node_count = self.nodes.len(),
            edge_count = self.edges.len(),
            "Starting Eulerian circuit computation"
        );

        // check if each node has even degree
        if !self
            .get_degrees()
            .values()
            .all(|&degree| degree.is_multiple_of(2))
        {
            return Err(GraphError::EulerianCircuitOddDegreeError);
        }

        // resulting sequence of node IDs in the Eulerian circuit
        let mut k: Vec<usize> = Vec::new();

        // create a mutable copy of the graph to modify during the algorithm
        let mut g = self.clone();

        let mut cycles = 0;

        while !g.edges.is_empty() {
            // find a starting node for the next cycle in the Eulerian circuit
            let u = g.try_find_start_node(&k)?;
            let mut v = u;

            // store the nodes in the current cycle
            let mut cycle = vec![u];

            loop {
                // find a remaining edge connected to v and remove it from the graph
                let w_pos = g
                    .edges
                    .iter()
                    .position(|e| e.u == v || e.v == v)
                    .ok_or(GraphError::EulerianCircuitDisconnectedGraphError)?;

                let w = g.edges.remove(w_pos);

                // find the other node connected by the edge w and add it to the cycle
                v = if w.u == v { w.v } else { w.u };
                cycle.push(v);

                // terminate the cycle when returning to the starting node u
                if v == u {
                    break;
                }
            }

            cycles += 1;
            tracing::trace!(
                cycle_start = u,
                cycle_length = cycle.len(),
                remaining_edges = g.edges.len(),
                "Eulerian subcycle computed"
            );

            // merge the cycle found into the resulting circuit k
            if k.is_empty() {
                k = cycle;
            } else {
                // find the position of the first node of the cycle in k
                let pos = k
                    .iter()
                    .position(|&node_id| node_id == cycle[0])
                    .ok_or(GraphError::EulerianCircuitDisconnectedGraphError)?;

                // insert the cycle into k at the position found
                k.splice(pos..=pos, cycle);
            }
        }

        tracing::debug!(
            circuit_length = k.len(),
            cycles,
            "Eulerian circuit computation completed"
        );

        Ok(k)
    }

    /// Tries to find a starting node for the Eulerian circuit in the graph.
    ///
    /// # Arguments
    /// * `circuit` - A reference to the current sequence of node IDs in the Eulerian circuit being constructed.
    ///
    /// # Returns
    /// * `Result<usize, GraphError>` - On success, returns the ID of a node that can be used as a starting point for the next cycle in the Eulerian circuit.
    fn try_find_start_node(&self, circuit: &[usize]) -> Result<usize, GraphError> {
        self.edges
            .iter()
            .find_map(|e| {
                if circuit.is_empty() || circuit.contains(&e.u) {
                    Some(e.u)
                } else if circuit.contains(&e.v) {
                    Some(e.v)
                } else {
                    None
                }
            })
            .ok_or(if circuit.is_empty() {
                GraphError::EulerianCircuitEmptyGraphError
            } else {
                GraphError::EulerianCircuitDisconnectedGraphError
            })
    }
}

impl PartialEq for Edge {
    fn eq(&self, other: &Self) -> bool {
        self.u == other.u && self.v == other.v && self.weight == other.weight
    }
}

impl PartialEq for Node {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id && self.x == other.x && self.y == other.y && self.z == other.z
    }
}
