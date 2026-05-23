//! This module defines data structures for representing graphs, including nodes, edges, and the graph itself.
//! It also includes helper functions to calculate distances between nodes.

use std::collections::HashMap;

use crate::distances::*;
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
}
