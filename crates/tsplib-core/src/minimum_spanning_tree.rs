//! This module provides functions to compute the minimum spanning tree (MST) of a TSP instance using different algorithms:
//! Kruskal's, Prim's, and Borůvka's algorithms. Each function takes a `TsplibInstance` as input and returns a `Graph`
//! containing the edges in the MST or an error if the MST cannot be computed.

use itertools::Itertools;

use crate::enums::MstComputationError;
use crate::models::{Edge, Graph, TsplibInstance};

/// A union-find (disjoint set) data structure for Kruskal's algorithm.
#[derive(Debug)]
struct UnionFind {
    /// The parent of each element in the union-find structure, where `parent[i]` is the parent of element `i + 1` (1-based indexing).
    parent: Vec<usize>,
    /// The rank of each element in the union-find structure, used for union by rank optimization.
    rank: Vec<usize>,
}

impl UnionFind {
    /// Creates a new `UnionFind` instance for `n` elements, initializing each element as its own parent and setting the rank to 0.
    ///
    /// # Arguments
    /// * `n` - The number of elements in the union-find structure.
    ///
    /// # Returns
    /// * `UnionFind` - A new instance of the union-find data structure.
    fn new(n: usize) -> Self {
        Self {
            parent: (1..=n).collect(), // 1-based indexing
            rank: vec![0; n],
        }
    }

    /// Finds the representative (root) of the set that contains element `u`, applying path compression for efficiency.
    ///
    /// # Arguments
    /// * `u` - The element for which to find the representative.
    ///
    /// # Returns
    /// * `usize` - The representative (root) of the set containing `u`.
    fn find(&mut self, u: usize) -> usize {
        if self.parent[u - 1] != u {
            self.parent[u - 1] = self.find(self.parent[u - 1]);
        }
        self.parent[u - 1]
    }

    /// Unites the sets containing elements `u` and `v` using union by rank.
    /// Returns `true` if the sets were united, or `false` if they were already in the same set.
    ///
    /// # Arguments
    /// * `u` - The first element to unite.
    /// * `v` - The second element to unite.
    ///
    /// # Returns
    /// * `bool` - `true` if the sets were united, `false` if they were already in the same set.
    fn union(&mut self, u: usize, v: usize) -> bool {
        // find the representatives (roots) of the sets containing `u` and `v`
        let root_u = self.find(u);
        let root_v = self.find(v);

        // if both elements are in the same set, they cannot be united
        if root_u == root_v {
            return false;
        }

        // union by rank: attach the smaller tree to the root of the larger tree
        let root_u_idx = root_u - 1;
        let root_v_idx = root_v - 1;

        if self.rank[root_u_idx] < self.rank[root_v_idx] {
            self.parent[root_u_idx] = root_v;
        } else if self.rank[root_u_idx] > self.rank[root_v_idx] {
            self.parent[root_v_idx] = root_u;
        } else {
            self.parent[root_v_idx] = root_u;
            self.rank[root_u_idx] += 1;
        }

        true
    }
}

/// Computes the minimum spanning tree (MST) of a TSP instance using Kruskal's algorithm and prints the resulting edges and their weights.
///
/// # Arguments
/// * `tsplib_instance` - The TSP instance for which to compute the MST.
///
/// # Returns
/// * `Result<Graph, ConversionError>` - Graph struct containing the edges in the MST, or an error if the MST cannot be computed.
pub fn try_get_mst_kruskal(tsplib_instance: &TsplibInstance) -> Result<Graph, MstComputationError> {
    tracing::debug!(
        node_count = tsplib_instance.nodes.len(),
        edge_candidates = tsplib_instance.nodes.len() * (tsplib_instance.nodes.len() - 1) / 2,
        "Starting Kruskal's MST computation",
    );

    // resulting edges of the MST
    let mut t: Vec<(usize, usize, i32)> = Vec::new();

    let n = tsplib_instance.nodes.len();

    // get edges of the triangular matrix without diagonal and sort them by distance in ascending order
    let edges = (0..(n - 1))
        .flat_map(|i| {
            ((i + 1)..n).map(move |j| {
                tsplib_instance
                    .try_get_distance(i + 1, j + 1)
                    .map(|distance| (i + 1, j + 1, distance))
            })
        })
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .sorted_by_key(|&(_, _, distance)| distance)
        .collect::<Vec<_>>();

    // initialize union-find structure for `n` nodes
    let mut uf = UnionFind::new(n);

    // iterate over sorted edges and add them to the MST if they connect two different components
    for (i, j, weight) in edges {
        if uf.union(i, j) {
            t.push((i, j, weight));

            if t.len() == n - 1 {
                break;
            }
        }
    }

    let total_weight: i64 = t.iter().map(|&(_, _, w)| i64::from(w)).sum();
    tracing::debug!(
        mst_edges = t.len(),
        total_weight,
        "Kruskal's MST computation completed"
    );

    Ok(Graph {
        nodes: tsplib_instance.nodes.clone(),
        edges: t
            .iter()
            .map(|&(u, v, weight)| Edge { u, v, weight })
            .collect(),
    })
}

/// Computes the minimum spanning tree (MST) of a TSP instance using Prim's algorithm starting from a specified node and returns the resulting edges and their weights.
///
/// # Arguments
/// * `tsplib_instance` - The TSP instance for which to compute the MST.
/// * `start_node` - The ID of the starting node for Prim's algorithm (1-based index).
/// # Returns
/// * `Result<Graph, ConversionError>` - Graph struct containing the edges in the MST,
///   or an error if the adjacency matrix is empty or the start node is invalid.
pub fn try_get_mst_prim(
    tsplib_instance: &TsplibInstance,
    start_node: usize,
) -> Result<Graph, MstComputationError> {
    tracing::debug!(
        node_count = tsplib_instance.nodes.len(),
        edge_candidates = tsplib_instance.nodes.len() * (tsplib_instance.nodes.len() - 1) / 2,
        start_node,
        "Starting Prim's MST computation",
    );

    let n = tsplib_instance.nodes.len();

    // check if start_node is valid
    if start_node == 0 || start_node > n {
        return Err(MstComputationError::PrimMstError(
            "Invalid start node provided".to_string(),
        ));
    }

    let mut in_mst = vec![false; n];
    // store the weight of the best edge connecting node v to the growing MST
    let mut best_weight = vec![i32::MAX; n];
    // store the parent of each node in the MST
    let mut parent: Vec<Option<usize>> = vec![None; n];

    // resulting edges of the MST
    let mut t = Vec::with_capacity(n - 1);

    best_weight[start_node - 1] = 0;

    for _ in 0..n {
        // find the node u that is not yet in the MST and has the smallest best_weight
        let u = (0..n)
            .filter(|&v| !in_mst[v])
            .min_by_key(|&v| best_weight[v])
            .ok_or_else(|| MstComputationError::PrimMstError("Disconnected graph".to_string()))?;

        in_mst[u] = true;

        if let Some(p) = parent[u] {
            t.push((p + 1, u + 1, best_weight[u]));
        }

        // update the best weights for nodes not yet in the MST
        for v in 0..n {
            if !in_mst[v] {
                // let weight = matrix[u][v];
                let weight = tsplib_instance.try_get_distance(u + 1, v + 1)?;

                if weight < best_weight[v] {
                    best_weight[v] = weight;
                    parent[v] = Some(u);
                }
            }
        }
    }

    let total_weight: i64 = t.iter().map(|&(_, _, w)| i64::from(w)).sum();
    tracing::debug!(
        mst_edges = t.len(),
        total_weight,
        "Prim's MST computation completed"
    );

    Ok(Graph {
        nodes: tsplib_instance.nodes.clone(),
        edges: t
            .iter()
            .map(|&(u, v, weight)| Edge { u, v, weight })
            .collect(),
    })
}

/// Computes the minimum spanning tree (MST) of a TSP instance using Borůvka's algorithm and returns the resulting edges and their weights.
///
/// # Arguments
/// * `tsplib_instance` - The TSP instance for which to compute the MST.
///
/// # Returns
/// * `Result<Graph, ConversionError>` - Graph struct containing the edges in the MST,
///   or an error if the MST cannot be computed.
pub fn try_get_mst_boruvka(tsplib_instance: &TsplibInstance) -> Result<Graph, MstComputationError> {
    // helper function to update the cheapest edge for a given component
    fn update_cheapest(cheapest: &mut [Option<Edge>], root: usize, edge: Edge) {
        if cheapest[root].is_none_or(|current| edge.weight < current.weight) {
            cheapest[root] = Some(edge);
        }
    }

    tracing::debug!(
        node_count = tsplib_instance.nodes.len(),
        edge_candidates = tsplib_instance.nodes.len() * (tsplib_instance.nodes.len() - 1) / 2,
        "Starting Borůvka's MST computation",
    );

    let n = tsplib_instance.nodes.len();

    let edges = (0..(n - 1))
        .flat_map(|i| {
            ((i + 1)..n).map(move |j| {
                tsplib_instance
                    .try_get_distance(i + 1, j + 1)
                    .map(|distance| Edge {
                        u: i + 1,
                        v: j + 1,
                        weight: distance,
                    })
            })
        })
        .collect::<Result<Vec<_>, _>>()?;

    // initialize union-find structure for `n` nodes
    let mut uf = UnionFind::new(n);

    // resulting edges of the MST
    let mut t = Vec::with_capacity(n - 1);

    // initially, each node is its own component
    let mut components = n;

    let mut rounds = 0;

    while components > 1 {
        rounds += 1;
        let mut cheapest = vec![None; n + 1];

        edges.iter().for_each(|&edge| {
            // find cheapest edge for each component

            let root_u = uf.find(edge.u);
            let root_v = uf.find(edge.v);

            if root_u != root_v {
                update_cheapest(&mut cheapest, root_u, edge);
                update_cheapest(&mut cheapest, root_v, edge);
            }
        });

        let before = components;

        // contract the cheapest edges and merge the components using union-find
        cheapest.into_iter().flatten().for_each(|edge| {
            if uf.union(edge.u, edge.v) {
                t.push(edge);
                components -= 1;
            }
        });

        // if no components were merged in an iteration
        // the graph is disconnected and an MST cannot be formed
        if components == before {
            return Err(MstComputationError::BoruvkaMstError(
                "Disconnected graph".to_string(),
            ));
        }
    }

    let total_weight: i64 = t.iter().map(|&e| i64::from(e.weight)).sum();
    tracing::debug!(
        mst_edges = t.len(),
        total_weight,
        rounds,
        "Borůvka's MST computation completed"
    );

    Ok(Graph {
        nodes: tsplib_instance.nodes.clone(),
        edges: t,
    })
}
