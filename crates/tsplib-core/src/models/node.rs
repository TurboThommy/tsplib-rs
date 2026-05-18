//! This module defines the `Node` struct, which represents a node in a graph with its coordinates and provides methods to calculate various types of distances between nodes.
use serde::Serialize;

use crate::distances::*;

/// A struct representing the node of a graph.
#[derive(Debug, Serialize)]
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
