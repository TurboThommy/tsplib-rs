//! This module implements the Edmonds' Blossom algorithm for finding a minimum weight perfect matching in a graph.
use std::collections::HashMap;

use tsplib_core::models::{Edge, Graph, TsplibInstance};

use crate::{PerfectMatchingAlgorithm, errors::MatcherError};

/// Factor by which original edge weights are multiplied so that the half-integral duals become integral.
const WEIGHT_SCALE: i64 = 2;

#[derive(Default)]
pub struct WeightedEdmondsMatching {}

/// The original (unshrunk) graph.
#[derive(Debug, Clone)]
struct BaseGraph {
    /// Number of original nodes.
    node_count: usize,
    /// Maps each original node index to its TSPLIB node ID.
    index_to_node_id: Vec<usize>,
    /// Scaled weight of every edge, keyed by edge_key.
    weights: HashMap<(usize, usize), i64>,
}

/// Records everything needed to expand a pseudonode back into its original cycle when the blossom is eventually expanded.
#[derive(Debug, Clone)]
struct BlossomData {
    /// The derived-node ids forming the shrunk circuit, in cyclic order as they
    /// were at shrink time. `cycle[0` is the base of the blossom.
    cycle: Vec<usize>,
    /// The dual offset `y_s` that was subtracted from the boundary edges of
    /// each circuit member `s` at shrink time (`c'_vw -= y_v`). Expansion adds
    /// it back (`c'_st += y_s`). Keyed by circuit-member derived-node id.
    dual_offset: HashMap<usize, i64>,
}

/// The mutable derived graph `G'` together with all per-node algorithm state.
///
/// One global dual vector `duals` holds y for *every* node id, original and
/// pseudonode alike — there is no separate blossom-dual map. Thanks to the
/// `c'_vw -= y_v` weight update applied on shrink, the slack of any derived
/// edge is uniformly `weight(u,v) - duals[u] - duals[v]`.
struct DerivedGraph<'a> {
    base: &'a BaseGraph,

    /// Descriptor for every node id ever created. Pseudonodes are appended;
    /// ids are never reused, so indices stay stable across shrink/expand.
    kind: Vec<NodeKind>,
    /// Whether a node id is currently a node of `G'`. Becomes false when the
    /// node is absorbed into a pseudonode (on shrink) or replaced by its
    /// circuit (on expand).
    active: Vec<bool>,

    /// Adjacency of the current derived graph, by active node id. Kept sorted
    /// for deterministic iteration.
    adjacency: Vec<Vec<usize>>,
    /// Current derived edge weights `c'`, keyed by `edge_key` over node ids.
    weights: HashMap<(usize, usize), i64>,
    /// For each current derived edge, an underlying base edge `(orig_u, orig_v)`
    /// that realises it, with `orig_u` inside the `u`-side and `orig_v` inside
    /// the `v`-side. Lets an augmenting path over `G'` be lifted down to
    /// original nodes when blossoms are expanded.
    edge_origin: HashMap<(usize, usize), (usize, usize)>,

    /// Dual value y_v for every node id (original or pseudonode).
    duals: Vec<i64>,
    /// Alternating-tree label for every node id.
    label: Vec<Label>,
    /// Tree parent pointer (in `G'`) for every node id; `None` for roots/free.
    parent: Vec<Option<usize>>,

    /// Matching over derived-node ids: `mate[v] == Some(w)` iff `vw` is in `M'`.
    mate: Vec<Option<usize>>,

    /// For each *original* base node, the id of the outermost active derived
    /// node currently containing it. Maps base nodes into the current `G'`.
    base_to_derived: Vec<usize>,
}

/// Alternating-tree label of a derived node. `B(T)` = `Even`, `A(T)` = `Odd`
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Label {
    /// Even node of the current alternating tree (`B(T)`).
    Even,
    /// Odd node of the current alternating tree (`A(T)`).
    Odd,
    /// Not currently in the tree.
    Free,
}

/// A node of the derived graph. Can either be an original vertex or a pseudonode created by shrinking an odd circuit (blossom).
#[derive(Debug, Clone)]
enum NodeKind {
    /// An original vertex carrying its base-graph index.
    Original(usize),
    /// A pseudonode formed by shrinking the recorded circuit.
    Blossom(BlossomData),
}

impl PerfectMatchingAlgorithm for WeightedEdmondsMatching {
    fn try_compute(
        &self,
        odd_vertices: &[usize],
        problem: &tsplib_core::models::TsplibInstance,
    ) -> Result<Vec<Edge>, MatcherError> {
        if !odd_vertices.len().is_multiple_of(2) {
            return Err(MatcherError::OddVertexCountError(odd_vertices.len()));
        }

        let mut odd_vertices = odd_vertices.to_vec();
        odd_vertices.sort_unstable();

        let _graph = try_build_complete_graph_for_vertices(&odd_vertices, problem)?;

        todo!("WeightedEdmondsMatching is not implemented yet")
    }
}

impl WeightedEdmondsMatching {
    pub fn new() -> Self {
        Self {}
    }
}

impl BaseGraph {
    /// Builds the base graph from the complete graph. All weights are scaled by `WEIGHT_SCALE`.
    ///
    /// # Arguments
    /// * `graph` - The complete graph containing the nodes and edges to be used for the matching algorithm.
    ///
    /// # Returns
    /// * `BaseGraph` - A new instance of `BaseGraph` containing the node count, mapping from indices to node IDs, and the scaled edge weights.
    fn from_graph(graph: &Graph) -> Self {
        let index_to_node_id = graph.nodes.iter().map(|node| node.id).collect::<Vec<_>>();

        let node_id_to_index = index_to_node_id
            .iter()
            .enumerate()
            .map(|(index, &node_id)| (node_id, index))
            .collect::<HashMap<_, _>>();

        let mut weights = HashMap::new();

        for edge in &graph.edges {
            let Some(&u) = node_id_to_index.get(&edge.u) else {
                continue;
            };
            let Some(&v) = node_id_to_index.get(&edge.v) else {
                continue;
            };
            if u == v {
                continue;
            }

            weights.insert(edge_key(u, v), WEIGHT_SCALE * i64::from(edge.weight));
        }

        Self {
            node_count: graph.nodes.len(),
            index_to_node_id,
            weights,
        }
    }

    /// Scaled weight of the edge between two vertices, if it exists in the graph.
    ///
    /// # Arguments
    /// * `u` - The index of the first vertex.
    /// * `v` - The index of the second vertex.
    ///
    /// # Returns
    /// * `Option<i64>` - The scaled weight of the edge if it exists, or `None` if there is no edge between the two vertices.
    fn weight(&self, u: usize, v: usize) -> Option<i64> {
        self.weights.get(&edge_key(u, v)).copied()
    }

    /// Maps an original node index back to its TSPLIB node ID.
    ///
    /// # Arguments
    /// * `index` - The index of the original node.
    ///
    /// # Returns
    /// * `Result<usize, MatcherError>` - The TSPLIB node ID corresponding to the given index, or an error if the index is out of bounds.
    fn node_id(&self, index: usize) -> Result<usize, MatcherError> {
        self.index_to_node_id
            .get(index)
            .copied()
            .ok_or(MatcherError::InvalidNodeIndex(
                index,
                self.index_to_node_id.len(),
            ))
    }

    /// Converts a final mate vector over original nodes into TSPLIBN edges undoing the scaling.
    ///
    /// # Arguments
    /// * `mate` - A slice where `mate[u]` gives the index of the vertex matched to vertex `u`, or `None` if `u` is unmatched.
    /// * `self` - The `BaseGraph` instance containing the mapping from indices to node IDs and the edge weights.
    ///
    /// # Returns
    /// * `Result<Vec<Edge>, MatcherError>` - A result containing a vector of `Edge` instances representing the matched edges
    ///   in terms of TSPLIB node IDs and original weights, or an error if the mate vector is invalid or if any edge is missing.
    fn try_matching_to_edge(&self, mate: &[Option<usize>]) -> Result<Vec<Edge>, MatcherError> {
        let mut edges = Vec::with_capacity(self.node_count / 2);

        for (u, &mate_u) in mate.iter().enumerate() {
            let Some(v) = mate_u else {
                return Err(MatcherError::InvalidAugmentingPath);
            };

            // To avoid duplicates, only consider edges where u < v.
            if u >= v {
                continue;
            }

            let scaled = self.weight(u, v).ok_or(MatcherError::MissingEdge(u, v))?;

            edges.push(Edge {
                u: self.node_id(u)?,
                v: self.node_id(v)?,
                weight: (scaled / WEIGHT_SCALE) as i32,
            });
        }

        Ok(edges)
    }
}

impl<'a> DerivedGraph<'a> {
    /// Builds the initial derived graph `G'` from the base graph `G`. Initially, `G'` is identical to `G`,
    /// with all original nodes active and no pseudonodes.
    ///
    /// # Arguments
    /// * `base` - A reference to the `BaseGraph` instance from which to build the derived graph.
    fn new(base: &'a BaseGraph) -> Self {
        let n = base.node_count;

        let mut adjacency = vec![Vec::new(); n];
        let mut weights = HashMap::new();
        let mut edge_origin = HashMap::new();

        for u in 0..n {
            for v in (u + 1)..n {
                if let Some(w) = base.weight(u, v) {
                    adjacency[u].push(v);
                    adjacency[v].push(u);
                    weights.insert((u, v), w);
                    edge_origin.insert((u, v), (u, v));
                }
            }
        }

        for neighbours in &mut adjacency {
            neighbours.sort_unstable();
        }

        Self {
            base,
            kind: (0..n).map(NodeKind::Original).collect(),
            active: vec![true; n],
            adjacency,
            weights,
            edge_origin,
            duals: vec![0; n],
            label: vec![Label::Free; n],
            parent: vec![None; n],
            mate: vec![None; n],
            base_to_derived: (0..n).collect(),
        }
    }

    /// Current `c'` weight of a derived edge if present.
    ///
    /// # Arguments
    /// * `u` - The index of the first vertex in the derived graph.
    /// * `v` - The index of the second vertex in the derived graph.
    ///
    /// # Returns
    /// * `Option<i64>` - The current weight of the edge between `u` and `v` in the derived graph, if it exists; otherwise, `None`.
    fn weight(&self, u: usize, v: usize) -> Option<i64> {
        self.weights.get(&edge_key(u, v)).copied()
    }

    /// Reduced cost / slack of a derived edge: `c'_uv - y_u - y_v`.
    ///
    /// # Arguments
    /// * `u` - The index of the first vertex in the derived graph.
    /// * `v` - The index of the second vertex in the derived graph.
    ///
    /// # Returns
    /// * `Option<i64>` - The slack of the edge between `u` and `v` in the derived graph, if it exists; otherwise, `None`.
    fn slack(&self, u: usize, v: usize) -> Option<i64> {
        self.weight(u, v).map(|w| w - self.duals[u] - self.duals[v])
    }

    /// Checks if a given vertex in the derived graph is a pseudonode (blossom) or an original vertex.
    ///
    /// # Arguments
    /// * `v` - The index of the vertex in the derived graph to check.
    ///
    /// # Returns
    /// * `bool` - `true` if the vertex is a pseudonode (blossom), `false` if it is an original vertex.
    fn is_pseudonode(&self, v: usize) -> bool {
        matches!(self.kind[v], NodeKind::Blossom(_))
    }

    /// Returns the total number of nodes (original and pseudonodes) currently in the derived graph.
    ///
    /// # Returns
    /// * `usize` - The total number of nodes in the derived graph, including both original vertices and pseudonodes (blossoms).
    fn total_nodes(&self) -> usize {
        self.kind.len()
    }

    /// Returns an iterator over the indices of the currently active nodes in the derived graph.
    ///
    /// # Returns
    /// * `impl Iterator<Item = usize>` - An iterator that yields the indices of the active nodes in the derived graph.
    ///   Active nodes are those that are not absorbed into a pseudonode (blossom) and are currently part of the graph structure.
    fn active_nodes(&self) -> impl Iterator<Item = usize> + '_ {
        (0..self.kind.len()).filter(move |&v| self.active[v])
    }

    /// Maps a base node index to the index of the outermost active derived node currently containing it.
    ///
    /// # Arguments
    /// * `base_node` - The index of the original node in the base graph.
    ///
    /// # Returns
    /// * `usize` - The index of the outermost active derived node in the derived graph that currently contains the given base node.
    ///   This mapping is crucial for navigating between the original graph and the derived graph, especially when handling blossoms and their expansions.
    fn derived_of_base(&self, base_node: usize) -> usize {
        self.base_to_derived[base_node]
    }
}

/// Helper function to create a consistent key for an edge between two vertices, regardless of their order.
///
/// # Arguments
/// * `u` - The index of the first vertex.
/// * `v` - The index of the second vertex.
///
/// # Returns
/// * `(usize, usize)` - A tuple representing the edge between the two vertices, with the smaller index first to ensure consistency.
fn edge_key(u: usize, v: usize) -> (usize, usize) {
    if u < v { (u, v) } else { (v, u) }
}

/// Builds a complete graph for the given vertices based on the distances in the TSP instance.
///
/// # Arguments
/// * `vertices` - A slice of vertex indices for which to build the complete graph.
/// * `problem` - The TSP instance containing the nodes and distance information.
///
/// # Returns
/// * `Result<Graph, MatcherError>` - A result containing the complete graph or
///   an error if any vertex is not found or if distance retrieval fails.
fn try_build_complete_graph_for_vertices(
    vertices: &[usize],
    problem: &TsplibInstance,
) -> Result<Graph, MatcherError> {
    let nodes = vertices
        .iter()
        .map(|&id| {
            problem
                .nodes
                .iter()
                .find(|node| node.id == id)
                .copied()
                .ok_or(MatcherError::NoMatchingCandidate(id))
        })
        .collect::<Result<Vec<_>, _>>()?;

    let mut edges = Vec::new();

    for i in 0..nodes.len() {
        for j in (i + 1)..vertices.len() {
            let u = vertices[i];
            let v = vertices[j];

            let weight = problem.try_get_distance(u, v)?;

            edges.push(Edge { u, v, weight });
        }
    }

    Ok(Graph { nodes, edges })
}

#[cfg(test)]
mod oracle_tests;
#[cfg(test)]
mod tests;
