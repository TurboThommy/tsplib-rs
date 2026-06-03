//! This module implements the Edmonds' Blossom algorithm for finding a minimum weight perfect matching in a graph.
use std::collections::{HashMap, HashSet, VecDeque};

use tsplib_core::models::Graph;

use crate::errors::MatcherError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Label {
    Even,
    Odd,
    Unlabeled,
}

#[derive(Debug, PartialEq, Eq, Clone)]
enum SearchResult {
    AugmentingPath(Vec<usize>),
    Blossom {
        cycle: Vec<usize>,
        base: usize,
        edge: (usize, usize),
    },
    None,
}

#[derive(Debug, Clone)]
struct Blossom {
    base: usize,
    cycle: Vec<usize>,
}

/// A struct to represent the state of the matching during the algorithm.
#[derive(Debug, Clone)]
struct MatchingState {
    /// The `mate` vector holds the index of the matched vertex for each vertex in the graph.
    /// If a vertex is unmatched, its entry is `None`.
    mate: Vec<Option<usize>>,
}

#[derive(Debug, Clone)]
struct ShrunkGraph {
    graph: EdmondsGraph,
    blossom_node: usize,
    original_to_shrunk: Vec<usize>,
    shrunk_to_original: Vec<Option<usize>>,
}

#[derive(Debug, Clone)]
struct EdmondsGraph {
    adjacency: Vec<Vec<usize>>,
    weights: HashMap<(usize, usize), i32>,
    index_to_node_id: Vec<usize>,
    node_id_to_index: HashMap<usize, usize>,
}

#[derive(Debug, Clone)]
struct DualState {
    vertex_duals: Vec<i64>,
}

#[derive(Debug, Clone)]
struct AlternatingTree {
    label: Vec<Label>,
    parent: Vec<Option<usize>>,
}

#[derive(Debug, Clone)]
struct TreeSearchResult {
    tree: AlternatingTree,
    result: SearchResult,
}

impl MatchingState {
    /// Creates a new `MatchingState` with the given number of vertices, initializing all vertices as unmatched.
    ///
    /// # Arguments
    /// * `node_count` - The number of vertices in the graph.
    fn new(node_count: usize) -> Self {
        Self {
            mate: vec![None; node_count],
        }
    }

    /// Checks if a vertex is exposed (unmatched) in the current matching state.
    ///
    /// # Arguments
    /// * `node` - The index of the vertex to check.
    ///
    /// # Returns
    /// * `true` if the vertex is unmatched, `false` otherwise.
    fn is_exposed(&self, node: usize) -> bool {
        self.mate[node].is_none()
    }

    /// Matches two vertices `u` and `v` by updating the `mate` vector accordingly.
    ///
    /// # Arguments
    /// * `u` - The index of the first vertex to match.
    /// * `v` - The index of the second vertex to match.
    fn match_edge(&mut self, u: usize, v: usize) {
        self.mate[u] = Some(v);
        self.mate[v] = Some(u);
    }

    /// Unmatches two vertices `u` and `v` by setting their entries in the `mate` vector to `None`.
    ///
    /// # Arguments
    /// * `u` - The index of the first vertex to unmatch.
    /// * `v` - The index of the second vertex to unmatch.
    fn unmatch_edge(&mut self, u: usize, v: usize) {
        self.mate[u] = None;
        self.mate[v] = None;
    }

    /// Augments the matching along a given path by toggling the matched and unmatched edges.
    ///
    /// # Arguments
    /// * `path` - A slice of vertex indices representing the path along which to augment the matching.
    fn augment_path(&mut self, path: &[usize]) {
        let old_mate = self.mate.clone();

        // Toggle the edges along the path in two phases to avoid conflicts when updating the `mate` vector.
        // First, unmatch the edges that are currently matched along the path.
        for window in path.windows(2) {
            let u = window[0];
            let v = window[1];

            if old_mate[u] == Some(v) {
                self.unmatch_edge(u, v);
            }
        }

        // Then, match the edges that are currently unmatched along the path.
        for window in path.windows(2) {
            let u = window[0];
            let v = window[1];

            if old_mate[u] != Some(v) {
                self.match_edge(u, v);
            }
        }
    }

    /// Checks if the current matching state is valid by ensuring that all matched vertices are properly paired
    /// and that no vertex is matched to itself.
    ///
    /// # Returns
    /// * `true` if the matching state is valid, `false` otherwise.
    fn is_valid(&self) -> bool {
        for (u, mate) in self.mate.iter().enumerate() {
            let Some(v) = mate else {
                continue;
            };

            if *v == u {
                return false;
            }

            if *v >= self.mate.len() {
                return false;
            }

            if self.mate[*v] != Some(u) {
                return false;
            }
        }

        true
    }

    /// Returns the cardinality of the matching, which is the number of matched edges in the current matching state.
    ///
    /// # Returns
    /// * `usize` representing the number of matched edges in the current matching state.
    fn cardinality(&self) -> usize {
        self.mate
            .iter()
            .enumerate()
            .filter(|(u, mate)| mate.is_some_and(|v| *u < v))
            .count()
    }
}

impl Blossom {
    /// Creates a new `Blossom` with the given base vertex and cycle of vertices.
    ///
    /// # Arguments
    /// * `base` - The index of the base vertex of the blossom.
    /// * `cycle` - A vector of vertex indices representing the cycle of the blossom.
    fn new(base: usize, cycle: Vec<usize>) -> Self {
        Self { base, cycle }
    }

    /// Checks if a given vertex is part of the blossom's cycle.
    ///
    /// # Arguments
    /// * `node` - The index of the vertex to check.
    ///
    /// # Returns
    /// * `true` if the vertex is part of the blossom's cycle, `false` otherwise.
    fn contains(&self, node: usize) -> bool {
        self.cycle.contains(&node)
    }

    /// Returns the index of a given vertex in the blossom's cycle if it is part of the cycle.
    ///
    /// # Arguments
    /// * `node` - The index of the vertex to find in the cycle.
    ///
    /// # Returns
    /// * `Some(usize)` containing the index of the vertex in the cycle if it is part of the cycle, or `None` if it is not.
    fn cycle_index(&self, node: usize) -> Option<usize> {
        self.cycle.iter().position(|&n| n == node)
    }

    /// Returns the paths along the blossom's cycle between two given vertices, if both vertices are part of the cycle.
    ///
    /// # Arguments
    /// * `from` - The index of the starting vertex in the cycle.
    /// * `to` - The index of the ending vertex in the cycle.
    ///
    /// # Returns
    /// * `Result<(Vec<usize>, Vec<usize>), MatcherError>` - A tuple containing the forward and backward paths along the cycle
    ///   between the two vertices, or an error if either vertex is not part of the cycle.
    fn cycle_paths_between(
        &self,
        from: usize,
        to: usize,
    ) -> Result<(Vec<usize>, Vec<usize>), MatcherError> {
        let from_index = self
            .cycle_index(from)
            .ok_or(MatcherError::NodeNotInBlossom(from))?;
        let to_index = self
            .cycle_index(to)
            .ok_or(MatcherError::NodeNotInBlossom(to))?;

        let n = self.cycle.len();

        // Construct the forward path from `from` to `to` along the cycle, wrapping around if necessary.
        let mut forward = Vec::new();
        let mut i = from_index;

        loop {
            forward.push(self.cycle[i]);

            if i == to_index {
                break;
            }

            i = (i + 1) % n;
        }

        // Construct the backward path from `to` to `from` along the cycle, wrapping around if necessary.
        let mut backward = Vec::new();
        let mut i = from_index;

        loop {
            backward.push(self.cycle[i]);

            if i == to_index {
                break;
            }

            i = (i + n - 1) % n;
        }

        Ok((forward, backward))
    }
}

impl EdmondsGraph {
    /// Creates a new `EdmondsGraph` from a given `Graph` by constructing the adjacency list and mapping between node IDs and indices.
    ///
    /// # Arguments
    /// * `graph` - A reference to the original `Graph` from which to create the `EdmondsGraph`.
    fn from_graph(graph: &Graph) -> Self {
        let index_to_node_id = graph.nodes.iter().map(|node| node.id).collect::<Vec<_>>();

        let node_id_to_index = index_to_node_id
            .iter()
            .enumerate()
            .map(|(index, &node_id)| (node_id, index))
            .collect::<HashMap<_, _>>();

        let mut adjacency = vec![Vec::new(); graph.nodes.len()];
        let mut weights = HashMap::new();

        for edge in &graph.edges {
            let Some(&u) = node_id_to_index.get(&edge.u) else {
                continue;
            };

            let Some(&v) = node_id_to_index.get(&edge.v) else {
                continue;
            };

            if !adjacency[u].contains(&v) {
                adjacency[u].push(v);
            }

            if !adjacency[v].contains(&u) {
                adjacency[v].push(u);
            }

            weights.insert(edge_key(u, v), edge.weight);
        }

        Self {
            adjacency,
            index_to_node_id,
            weights,
            node_id_to_index,
        }
    }

    /// Creates an iterator over the neighbors of a given node in the graph.
    ///
    /// # Arguments
    /// * `node` - The index of the node whose neighbors are to be returned.
    ///
    /// # Returns
    /// * `impl Iterator<Item = usize>` - An iterator over the indices of the neighboring vertices.
    fn neighbors(&self, node: usize) -> impl Iterator<Item = usize> + '_ {
        self.adjacency[node].iter().copied()
    }

    /// Returns the weight of the edge between two given vertices `u` and `v` if it exists in the graph.
    ///
    /// # Arguments
    /// * `u` - The index of the first vertex.
    /// * `v` - The index of the second vertex.
    ///
    /// # Returns
    /// * `Option<i32>` - The weight of the edge between `u` and `v` if it exists, or `None` if no such edge exists in the graph.
    fn weight(&self, u: usize, v: usize) -> Option<i32> {
        self.weights.get(&edge_key(u, v)).copied()
    }

    /// Attempts to determine if the edge between two given vertices `u` and `v` is tight with respect to the current dual values.
    ///
    /// # Arguments
    /// * `duals` - A reference to the current `DualState` containing the dual values for the vertices.
    /// * `u` - The index of the first vertex.
    /// * `v` - The index of the second vertex.
    ///
    /// # Returns
    /// * `Result<bool, MatcherError>` - `Ok(true)` if the edge between `u` and `v` is tight (i.e., its slack is zero), `Ok(false)` if it is not tight,
    ///   or an error if the vertex indices are invalid or if the edge does not exist in the graph.
    fn try_is_tight(&self, duals: &DualState, u: usize, v: usize) -> Result<bool, MatcherError> {
        Ok(duals.try_slack(self, u, v)? == 0)
    }

    /// Attempts to retrieve the tight neighbors of a given vertex based on the current dual values, returning an error if the vertex index is invalid.
    ///
    /// # Arguments
    /// * `duals` - A reference to the current `DualState` containing the dual values for the vertices.
    /// * `node` - The index of the vertex for which to retrieve the tight neighbors.
    ///
    /// # Returns
    /// * `Result<Vec<usize>, MatcherError>` - A vector of indices of the tight neighbors of the given vertex if the vertex index is valid,
    ///   or an error if the vertex index is invalid or if any edge does not exist in the graph.
    fn try_tight_neighbors(
        &self,
        duals: &DualState,
        node: usize,
    ) -> Result<Vec<usize>, MatcherError> {
        let neighbors = self
            .adjacency
            .get(node)
            .ok_or(MatcherError::InvalidNodeIndex(node, self.adjacency.len()))?;

        neighbors
            .iter()
            .copied()
            .filter_map(|neighbor| match self.try_is_tight(duals, node, neighbor) {
                Ok(true) => Some(Ok(neighbor)),
                Ok(false) => None,
                Err(err) => Some(Err(err)),
            })
            .collect()
    }
}

impl DualState {
    /// Creates a new `DualState` with the given number of vertices, initializing all vertex duals to zero.
    ///
    /// # Arguments
    /// * `node_count` - The number of vertices in the graph for which to initialize the dual state.
    fn new(node_count: usize) -> Self {
        Self {
            vertex_duals: vec![0; node_count],
        }
    }

    /// Returns the dual value for a given vertex if the vertex index is valid.
    ///
    /// # Arguments
    /// * `node` - The index of the vertex for which to retrieve the dual value.
    ///
    /// # Returns
    /// * `Option<i64>` - The dual value for the vertex if the index is valid, or `None` if the vertex index is out of bounds.
    fn get(&self, node: usize) -> Option<i64> {
        self.vertex_duals.get(node).copied()
    }

    /// Attempts to retrieve the dual value for a given vertex, returning an error if the vertex index is out of bounds.
    ///
    /// # Arguments
    /// * `node` - The index of the vertex for which to retrieve the dual value.
    ///
    /// # Returns
    /// * `Result<i64, MatcherError>` - The dual value for the vertex if the index is valid, or an error if the vertex index is out of bounds.
    fn try_get(&self, node: usize) -> Result<i64, MatcherError> {
        self.vertex_duals
            .get(node)
            .copied()
            .ok_or(MatcherError::InvalidNodeIndex(
                node,
                self.vertex_duals.len(),
            ))
    }

    /// Attempts to set the dual value for a given vertex, returning an error if the vertex index is out of bounds.
    ///
    /// # Arguments
    /// * `node` - The index of the vertex for which to set the dual value.
    /// * `value` - The dual value to set for the vertex.
    ///
    /// # Returns
    /// * `Result<(), MatcherError>` - `Ok(())` if the dual value was successfully set, or an error if the vertex index is invalid.
    fn try_set(&mut self, node: usize, value: i64) -> Result<(), MatcherError> {
        let len = self.vertex_duals.len();

        let dual = self
            .vertex_duals
            .get_mut(node)
            .ok_or(MatcherError::InvalidNodeIndex(node, len))?;

        *dual = value;

        Ok(())
    }

    /// Attempts to add a delta value to the dual value of a given vertex, returning an error if the vertex index is out of bounds.
    ///
    /// # Arguments
    /// * `node` - The index of the vertex for which to add the delta value to the dual.
    /// * `delta` - The value to add to the dual of the vertex.
    ///
    /// # Returns
    /// * `Result<(), MatcherError>` - `Ok(())` if the delta value was successfully added to the dual, or an error if the vertex index is invalid.
    fn try_add(&mut self, node: usize, delta: i64) -> Result<(), MatcherError> {
        let len = self.vertex_duals.len();

        let dual = self
            .vertex_duals
            .get_mut(node)
            .ok_or(MatcherError::InvalidNodeIndex(node, len))?;

        *dual += delta;

        Ok(())
    }

    /// Attempts to calculate the slack of an edge between two vertices `u` and `v` based on the current dual values and the weight of the edge,
    /// returning an error if the vertex indices are out of bounds or if the edge does not exist in the graph.
    ///
    /// # Arguments
    /// * `graph` - A reference to the graph represented as an `EdmondsGraph` from which to retrieve the edge weight.
    /// * `u` - The index of the first vertex of the edge for which to calculate the slack.
    /// * `v` - The index of the second vertex of the edge for which to calculate the slack.
    ///
    /// # Returns
    /// * `Result<i64, MatcherError>` - The calculated slack of the edge if the vertex indices are valid and the edge exists,
    ///   or an error if the vertex indices are invalid or the edge is missing from the graph.
    fn try_slack(&self, graph: &EdmondsGraph, u: usize, v: usize) -> Result<i64, MatcherError> {
        let weight = graph.weight(u, v).ok_or(MatcherError::MissingEdge(u, v))?;

        // The slack of an edge is calculated as the weight of the edge minus the sum of the dual values of its endpoints.
        // Scale it by 2 to avoid dealing with fractions, since the dual values can be half-integers in the context of the blossom algorithm.
        Ok(2 * i64::from(weight) - self.try_get(u)? - self.try_get(v)?)
    }
}

impl AlternatingTree {
    fn even_vertices(&self) -> Vec<usize> {
        self.label
            .iter()
            .enumerate()
            .filter_map(|(node, label)| (*label == Label::Even).then_some(node))
            .collect()
    }

    fn odd_vertices(&self) -> Vec<usize> {
        self.label
            .iter()
            .enumerate()
            .filter_map(|(node, label)| (*label == Label::Odd).then_some(node))
            .collect()
    }
}

/// Reconstructs the path from the root to the target vertex using the parent pointers.
///
/// # Arguments
/// * `root` - The index of the root vertex from which the search started.
/// * `target` - The index of the target vertex to which the path is to be reconstructed.
/// * `parent` - A slice of `Option<usize>` representing the parent pointers for each vertex in the search tree.
///
/// # Returns
/// * `Result<Vec<usize>, MatcherError>` - A vector of vertex indices representing the path from the root to the target vertex,
///   or an error if the path is not connected to the root.
fn try_reconstruct_path(
    root: usize,
    target: usize,
    parent: &[Option<usize>],
) -> Result<Vec<usize>, MatcherError> {
    let mut path = vec![target];
    let mut current = target;

    while current != root {
        current = parent
            .get(current)
            .copied()
            .flatten()
            .ok_or(MatcherError::PathReconstructionError)?;

        path.push(current);
    }

    path.reverse();
    Ok(path)
}

/// Searches for an augmenting path in the graph starting from the given root vertex using a breadth-first search approach.
///
/// # Arguments
/// * `graph` - A reference to the graph represented as an `EdmondsGraph`.
/// * `matching` - A reference to the current state of the matching.
/// * `root` - The index of the root vertex from which to start the search for an augmenting path.
///
/// # Returns
/// * `Option<Vec<usize>>` - An optional vector of vertex indices representing the augmenting path found.
///   If no augmenting path exists, returns `None`.
fn search_alternating_tree(
    graph: &EdmondsGraph,
    matching: &MatchingState,
    root: usize,
) -> Result<SearchResult, MatcherError> {
    let node_count = graph.adjacency.len();

    // Initialize the label and parent vectors for the breadth-first search.
    let mut label = vec![Label::Unlabeled; node_count];
    let mut parent: Vec<Option<usize>> = vec![None; node_count];
    let mut queue = VecDeque::new();

    // Start the search from the root vertex, labeling it as even and adding it to the queue.
    label[root] = Label::Even;
    queue.push_back(root);

    // Perform a breadth-first search to find an augmenting path.
    while let Some(u) = queue.pop_front() {
        for v in graph.neighbors(u) {
            match label[v] {
                Label::Unlabeled => {
                    parent[v] = Some(u);

                    // If vertex `v` is exposed, an augmenting path from the root to `v` was found.
                    if matching.is_exposed(v) {
                        return Ok(SearchResult::AugmentingPath(try_reconstruct_path(
                            root, v, &parent,
                        )?));
                    }

                    // If vertex `v` is not exposed, label it as odd and label its mate as even,
                    // then add the mate to the queue for further exploration.
                    let mate = matching
                        .mate
                        .get(v)
                        .copied()
                        .flatten()
                        .ok_or(MatcherError::MissingMate(v))?;
                    label[v] = Label::Odd;
                    label[mate] = Label::Even;
                    parent[mate] = Some(v);
                    queue.push_back(mate);
                }

                // Blossom path
                Label::Even => {
                    // Ignore self-loops
                    if u == v {
                        continue;
                    }

                    if let Some(lca) = find_lca(u, v, &parent) {
                        let cycle = try_reconstruct_blossom_cycle(u, v, lca, &parent)?;

                        return Ok(SearchResult::Blossom {
                            cycle,
                            base: lca,
                            edge: (u, v),
                        });
                    }
                }

                Label::Odd => {}
            }
        }
    }

    Ok(SearchResult::None)
}

/// Finds the least common ancestor (LCA) of two vertices in the search tree defined by the parent pointers.
///
/// # Arguments
/// * `u` - The index of the first vertex.
/// * `v` - The index of the second vertex.
/// * `parent` - A slice of `Option<usize>` representing the parent pointers for each vertex in the search tree.
///
/// # Returns
/// * `Option<usize>` - The index of the least common ancestor of `u` and `v`, or `None` if no common ancestor exists.
fn find_lca(u: usize, v: usize, parent: &[Option<usize>]) -> Option<usize> {
    let mut ancestors = HashSet::new();

    // Traverse the path from `u` to the root, adding each vertex to the set of ancestors.
    let mut current = Some(u);
    while let Some(node) = current {
        ancestors.insert(node);
        current = parent[node];
    }

    // Traverse the path from `v` to the root, checking if any vertex is in the set of ancestors.
    let mut current = Some(v);
    while let Some(node) = current {
        if ancestors.contains(&node) {
            return Some(node);
        }
        current = parent[node];
    }

    None
}

/// Reconstructs the cycle formed by a blossom when an edge is added between two vertices `u` and `v` that are both labeled as even.
///
/// # Arguments
/// * `u` - The index of the first vertex involved in the blossom.
/// * `v` - The index of the second vertex involved in the blossom.
/// * `lca` - The index of the least common ancestor of `u` and `v` in the search tree.
/// * `parent` - A slice of `Option<usize>` representing the parent pointers for each vertex in the search tree.
///
/// # Returns
/// * `Result<Vec<usize>, MatcherError>` - A vector of vertex indices representing the cycle formed by the blossom,
///   or an error if the cycle cannot be reconstructed due to connectivity issues with
fn try_reconstruct_blossom_cycle(
    u: usize,
    v: usize,
    lca: usize,
    parent: &[Option<usize>],
) -> Result<Vec<usize>, MatcherError> {
    // Reconstruct the path from `u` to `lca`
    let mut left = Vec::new();
    let mut current = u;

    while current != lca {
        left.push(current);
        current = parent
            .get(current)
            .copied()
            .flatten()
            .ok_or(MatcherError::NodeNotConnectedToLca(current, lca))?;
    }

    left.push(lca);

    // Reconstruct the path from `v` to `lca`
    let mut right = Vec::new();
    let mut current = v;

    while current != lca {
        right.push(current);
        current = parent
            .get(current)
            .copied()
            .flatten()
            .ok_or(MatcherError::NodeNotConnectedToLca(current, lca))?;
    }

    // The path from `v` to `lca` is reversed to maintain the correct order
    // when combining with the path from `u` to `lca`.
    right.reverse();

    // Combine the two paths to form the cycle of the blossom.
    left.extend(right);
    Ok(left)
}

/// Shrinks the graph by contracting the blossom into a single vertex, creating a new graph representation that reflects the contraction.
///
/// # Arguments
/// * `graph` - A reference to the original graph.
/// * `blossom` - A reference to the `Blossom` struct representing the blossom to be contracted.
///
/// # Returns
/// * `ShrunkGraph` - A struct containing the new graph representation after shrinking the blossom,
///   the index of the new vertex representing the blossom,and a mapping from original vertices to
///   their corresponding vertices in the shrunk graph.
fn shrink_graph(graph: &EdmondsGraph, blossom: &Blossom) -> ShrunkGraph {
    let node_count = graph.adjacency.len();

    let mut original_to_shrunk = vec![usize::MAX; node_count];
    let mut shrunk_to_original = Vec::new();

    // map all non-blossom nodes to compact shrunk indices
    for node in 0..node_count {
        if blossom.contains(node) {
            continue;
        }

        let shrunk_node = shrunk_to_original.len();
        original_to_shrunk[node] = shrunk_node;
        shrunk_to_original.push(Some(node));
    }

    // one compact pseudonode for the blossom
    let blossom_node = shrunk_to_original.len();

    for &node in &blossom.cycle {
        original_to_shrunk[node] = blossom_node;
    }

    shrunk_to_original.push(None);

    let mut adjacency = vec![Vec::new(); shrunk_to_original.len()];
    let mut weights: HashMap<(usize, usize), i32> = HashMap::new();

    // Iterate over the edges in the original graph and add corresponding edges to the shrunk graph,
    // ensuring that edges within the blossom are not included in the shrunk graph.
    for u in 0..node_count {
        for v in graph.neighbors(u) {
            // Map the original vertices `u` and `v` to their corresponding vertices in the shrunk graph.
            let su = original_to_shrunk[u];
            let sv = original_to_shrunk[v];

            if su == sv {
                continue;
            }

            // Add edges to the shrunk graph, ensuring that duplicate edges are not added.
            if !adjacency[su].contains(&sv) {
                adjacency[su].push(sv);
            }

            if !adjacency[sv].contains(&su) {
                adjacency[sv].push(su);
            }

            let Some(weight) = graph.weight(u, v) else {
                continue;
            };

            let key = edge_key(su, sv);

            match weights.get_mut(&key) {
                // If an edge between `su` and `sv` already exists in the shrunk graph,
                // update its weight to be the minimum of the existing weight and the new weight.
                Some(existing) => {
                    *existing = (*existing).min(weight);
                }
                // If no edge between `su` and `sv` exists in the shrunk graph, add a new entry to the weights map
                // with the weight of the edge from the original graph.
                None => {
                    weights.insert(key, weight);
                }
            }
        }
    }

    let index_to_node_id = (0..adjacency.len()).collect::<Vec<_>>();

    let node_id_to_index = index_to_node_id
        .iter()
        .enumerate()
        .map(|(index, &node_id)| (node_id, index))
        .collect::<HashMap<_, _>>();

    let graph = EdmondsGraph {
        adjacency,
        weights,
        index_to_node_id,
        node_id_to_index,
    };

    ShrunkGraph {
        graph,
        blossom_node,
        original_to_shrunk,
        shrunk_to_original,
    }
}

/// Shrinks the matching state by mapping the matched edges from the original graph to the shrunk graph,
/// ensuring that edges within the blossom are not included in the shrunk matching.
///
/// # Arguments
/// * `matching` - A reference to the current state of the matching in the original graph.
/// * `blossom` - A reference to the `Blossom` struct representing the blossom that has been contracted.
/// * `shrunk` - A reference to the `ShrunkGraph` struct containing the new graph representation after shrinking the blossom.
///
/// # Returns
/// * `MatchingState` - A new `MatchingState` representing the matching in the shrunk graph,
///   with edges mapped from the original graph and edges within the blossom excluded.
fn shrink_matching(matching: &MatchingState, shrunk: &ShrunkGraph) -> MatchingState {
    let mut shrunk_matching = MatchingState::new(shrunk.graph.adjacency.len());

    // Iterate over the matched edges in the original matching and map them to the shrunk graph,
    for u in 0..matching.mate.len() {
        // Skip unmatched vertices
        let Some(v) = matching.mate[u] else {
            continue;
        };

        // avoid processing duplicates
        if u > v {
            continue;
        }

        // Map the original vertices `u` and `v` to their corresponding vertices in the shrunk graph.
        let su = shrunk.original_to_shrunk[u];
        let sv = shrunk.original_to_shrunk[v];

        // remove matching edges inside the blossom
        if su == sv {
            continue;
        }

        // Match the corresponding vertices in the shrunk graph.
        shrunk_matching.match_edge(su, sv);
    }

    shrunk_matching
}

/// Attempts to find an augmenting path in the graph using Edmonds' algorithm,
/// which includes handling blossoms by shrinking them and recursively searching for augmenting paths in the shrunk graph.
///
/// # Arguments
/// * `graph` - A reference to the graph represented as an `EdmondsGraph`.
/// * `matching` - A reference to the current state of the matching.
/// * `root` - The index of the root vertex from which to start the search for an augmenting path.
///
/// # Returns
/// * `Result<Option<Vec<usize>>, MatcherError>` - An optional vector of vertex indices representing the augmenting path found,
///   or `None` if no augmenting path exists, or an error if blossom expansion is not implemented.
fn try_find_augmenting_path_edmonds(
    graph: &EdmondsGraph,
    matching: &MatchingState,
    root: usize,
) -> Result<Option<Vec<usize>>, MatcherError> {
    match search_alternating_tree(graph, matching, root)? {
        SearchResult::AugmentingPath(path) => Ok(Some(path)),

        SearchResult::None => Ok(None),

        SearchResult::Blossom { cycle, base, .. } => {
            let blossom = Blossom::new(base, cycle);

            let shrunk = shrink_graph(graph, &blossom);
            let shrunk_matching = shrink_matching(matching, &shrunk);

            let shrunk_root = shrunk.original_to_shrunk[root];

            let shrunk_path =
                try_find_augmenting_path_edmonds(&shrunk.graph, &shrunk_matching, shrunk_root)?;

            match shrunk_path {
                Some(path) => {
                    let expanded_path =
                        try_expand_path_through_blossom(&path, graph, &shrunk, &blossom, matching)?;

                    Ok(Some(expanded_path))
                }
                None => Ok(None),
            }
        }
    }
}

fn is_alternating_path(path: &[usize], matching: &MatchingState) -> bool {
    if path.len() < 2 {
        return true;
    }

    let first_edge_is_matched = matching.mate[path[0]] == Some(path[1]);

    for (index, window) in path.windows(2).enumerate() {
        let u = window[0];
        let v = window[1];

        let is_matched = matching.mate[u] == Some(v);

        if index.is_multiple_of(2) {
            if is_matched != first_edge_is_matched {
                return false;
            }
        } else if is_matched == first_edge_is_matched {
            return false;
        }
    }

    true
}

/// Attempts to choose an alternating path along the blossom's cycle between two given vertices,
/// ensuring that the chosen path is valid according to the current matching state.
///
/// # Arguments
/// * `blossom` - A reference to the `Blossom` struct representing the blossom containing the cycle.
/// * `from` - The index of the starting vertex in the blossom's cycle from which to start the path.
/// * `to` - The index of the ending vertex in the blossom's cycle to which to end the path.
/// * `matching` - A reference to the current state of the matching.
///
/// # Returns
/// * `Result<Vec<usize>, MatcherError>` - A vector of vertex indices representing the chosen alternating path along the blossom's cycle,
///   or an error if no valid alternating path exists between the entry and exit vertices in the blossom's cycle.
fn try_choose_alternating_blossom_path(
    blossom: &Blossom,
    from: usize,
    to: usize,
    matching: &MatchingState,
) -> Result<Vec<usize>, MatcherError> {
    let (forward, backward) = blossom.cycle_paths_between(from, to)?;

    match (
        is_alternating_path(&forward, matching),
        is_alternating_path(&backward, matching),
    ) {
        (true, false) => Ok(forward),
        (false, true) => Ok(backward),
        (true, true) => Ok(forward),
        (false, false) => Err(MatcherError::NoAlternatingBlossomPath(from, to)),
    }
}

/// Attempts to choose an alternating path along the blossom's cycle between two given vertices,
/// ensuring that the chosen path is valid according to the current matching state.
///
/// # Arguments
/// * `blossom` - A reference to the `Blossom` struct representing the blossom containing the cycle.
/// * `entry` - The index of the entry vertex in the blossom's cycle from which to start the path.
/// * `exit` - The index of the exit vertex in the blossom's cycle to which to end the path.
/// * `matching` - A reference to the current state of the matching.
///
/// # Returns
/// * `Result<Vec<usize>, MatcherError>` - A vector of vertex indices representing the chosen alternating path along the blossom's cycle,
///   or an error if no valid alternating path exists between the entry and exit vertices in the blossom's cycle.
fn try_expand_blossom_node(
    blossom: &Blossom,
    entry: usize,
    exit: usize,
    matching: &MatchingState,
) -> Result<Vec<usize>, MatcherError> {
    try_choose_alternating_blossom_path(blossom, entry, exit, matching)
}

/// Finds an original edge in the graph that connects an external vertex to any vertex in the blossom,
/// which is necessary for reconstructing the path when expanding a blossom.
///
/// # Arguments
/// * `graph` - A reference to the original graph represented as an `EdmondsGraph`.
/// * `blossom` - A reference to the `Blossom` struct representing the blossom whose cycle is being expanded.
/// * `external` - The index of the external vertex that is connected to the blossom.
///
/// # Returns
/// * `Option<usize>` - The index of the vertex in the blossom that is connected to the external vertex, or `None` if no such edge exists.
fn find_original_edge_to_blossom(
    graph: &EdmondsGraph,
    blossom: &Blossom,
    external: usize,
) -> Option<usize> {
    graph
        .neighbors(external)
        .find(|&neighbor| blossom.contains(neighbor))
}

/// Attempts to expand an augmenting path that goes through a blossom by replacing the blossom node in
/// the path with a valid alternating path along the blossom's cycle.
///
/// # Arguments
/// * `path` - A slice of vertex indices representing the path that goes through the blossom, where one of the vertices is the blossom node.
/// * `graph` - A reference to the original graph represented as an `EdmondsGraph`.
/// * `shrunk` - A reference to the `ShrunkGraph` struct containing the new graph representation after shrinking the blossom.
/// * `blossom` - A reference to the `Blossom` struct representing the blossom that is being expanded.
/// * `matching` - A reference to the current state of the matching in the original graph.
///
/// # Returns
/// * `Result<Vec<usize>, MatcherError>` - A vector of vertex indices representing the expanded path with the blossom node
///   replaced by a valid alternating path along the blossom's cycle, or an error if the path cannot be expanded due to issues
///   with mapping shrunk nodes to original nodes or finding valid paths through the blossom.
fn try_expand_path_through_blossom(
    path: &[usize],
    graph: &EdmondsGraph,
    shrunk: &ShrunkGraph,
    blossom: &Blossom,
    matching: &MatchingState,
) -> Result<Vec<usize>, MatcherError> {
    // Find the position of the blossom node in the path. If the blossom node is not present,
    // map the path back to original nodes and return it.
    let Some(blossom_pos) = path.iter().position(|&node| node == shrunk.blossom_node) else {
        return path
            .iter()
            .map(|&node| {
                shrunk
                    .shrunk_to_original
                    .get(node)
                    .copied()
                    .flatten()
                    .ok_or(MatcherError::ShrunkNodeNotMapped(node))
            })
            .collect();
    };

    // Get the previous and next nodes in the path adjacent to the blossom node
    let prev = if blossom_pos > 0 {
        path.get(blossom_pos - 1).copied()
    } else {
        None
    };
    let next = path.get(blossom_pos + 1).copied();

    let blossom_path = match (prev, next) {
        (Some(prev), Some(next)) => {
            // Map the previous and next nodes from the shrunk graph back to their corresponding original nodes in the original graph.
            let prev_original = shrunk
                .shrunk_to_original
                .get(prev)
                .copied()
                .flatten()
                .ok_or(MatcherError::ShrunkNodeNotMapped(prev))?;

            let next_original = shrunk
                .shrunk_to_original
                .get(next)
                .copied()
                .flatten()
                .ok_or(MatcherError::ShrunkNodeNotMapped(next))?;

            // Find the edges in the original graph that connect the previous and next nodes to any vertex in the blossom,
            // which will serve as the entry and exit points for expanding the blossom.
            let entry = find_original_edge_to_blossom(graph, blossom, prev_original)
                .ok_or(MatcherError::NoEdgeIntoBlossom(prev_original))?;
            let exit = find_original_edge_to_blossom(graph, blossom, next_original)
                .ok_or(MatcherError::NoEdgeIntoBlossom(next_original))?;

            // Choose a valid alternating path along the blossom's cycle between the entry and exit vertices,
            // which will replace the blossom node in the path.
            try_expand_blossom_node(blossom, entry, exit, matching)?
        }

        (None, Some(next)) => {
            // Map the next node from the shrunk graph back to its corresponding original node in the original graph.
            let next_original = shrunk
                .shrunk_to_original
                .get(next)
                .copied()
                .flatten()
                .ok_or(MatcherError::ShrunkNodeNotMapped(next))?;

            // Find the edge in the original graph that connects the next node to any vertex in the blossom,
            // which will serve as the exit point for expanding the blossom.
            let exit = find_original_edge_to_blossom(graph, blossom, next_original)
                .ok_or(MatcherError::NoEdgeIntoBlossom(next_original))?;

            // Choose a valid alternating path along the blossom's cycle between the base of the blossom and the exit vertex,
            // which will replace the blossom node in the path.
            try_expand_blossom_node(blossom, blossom.base, exit, matching)?
        }

        (Some(prev), None) => {
            // Map the previous node from the shrunk graph back to its corresponding original node in the original graph.
            let prev_original = shrunk
                .shrunk_to_original
                .get(prev)
                .copied()
                .flatten()
                .ok_or(MatcherError::ShrunkNodeNotMapped(prev))?;

            // Find the edge in the original graph that connects the previous node to any vertex in the blossom,
            // which will serve as the entry point for expanding the blossom.
            let entry = find_original_edge_to_blossom(graph, blossom, prev_original)
                .ok_or(MatcherError::NoEdgeIntoBlossom(prev_original))?;

            // Choose a valid alternating path along the blossom's cycle between the entry vertex and the base of the blossom,
            // which will replace the blossom node in the path.
            try_expand_blossom_node(blossom, entry, blossom.base, matching)?
        }

        (None, None) => vec![blossom.base],
    };

    let mut expanded = Vec::new();

    // Map the portion of the path before the blossom node back to original nodes and add it to the expanded path.
    for &node in &path[..blossom_pos] {
        let original = shrunk
            .shrunk_to_original
            .get(node)
            .copied()
            .flatten()
            .ok_or(MatcherError::ShrunkNodeNotMapped(node))?;

        expanded.push(original);
    }

    expanded.extend(blossom_path);

    // Map the portion of the path after the blossom node back to original nodes and add it to the expanded path.
    for &node in &path[(blossom_pos + 1)..] {
        let original = shrunk
            .shrunk_to_original
            .get(node)
            .copied()
            .flatten()
            .ok_or(MatcherError::ShrunkNodeNotMapped(node))?;

        expanded.push(original);
    }

    Ok(expanded)
}

/// Attempts to compute a maximum matching in the graph using Edmonds' algorithm by repeatedly searching
/// for augmenting paths and augmenting the matching until no more augmenting paths can be found.
///
/// # Arguments
/// * `graph` - A reference to the graph represented as an `EdmondsGraph` for which to compute the maximum matching.
///
/// # Returns
/// * `Result<MatchingState, MatcherError>` - The final state of the matching after no more augmenting paths can be found,
///   or an error if any issues occur during the search for augmenting paths or augmentation process.
fn try_compute_maximum_matching(graph: &EdmondsGraph) -> Result<MatchingState, MatcherError> {
    let mut matching = MatchingState::new(graph.adjacency.len());

    loop {
        let mut augmented = false;

        for root in 0..graph.adjacency.len() {
            if !matching.is_exposed(root) {
                continue;
            }

            let Some(path) = try_find_augmenting_path_edmonds(graph, &matching, root)? else {
                continue;
            };

            matching.augment_path(&path);
            augmented = true;
            break;
        }

        if !augmented {
            break;
        }
    }

    Ok(matching)
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

/// Searches for an augmenting path in the graph starting from the given root vertex using a breadth-first search approach,
/// while also handling the detection of blossoms when two even-labeled vertices are connected by an edge
/// and returning the appropriate search result based on whether an augmenting path is found, a blossom is detected, or no path exists.
///
/// # Arguments
/// * `graph` - A reference to the graph represented as an `EdmondsGraph`.
/// * `duals` - A reference to the current state of the dual variables for the vertices in the graph,
///   which is used to determine tight edges during the search for augmenting paths.
/// * `matching` - A reference to the current state of the matching.
/// * `root` - The index of the root vertex from which to start the search for an augmenting path.
///
/// # Returns
/// * `Result<SearchResult, MatcherError>` - A `SearchResult` enum indicating whether an augmenting path was found,
///   a blossom was detected, or no path exists, or an error if any issues occur during the search process,
///   such as missing mates or invalid node indices.
fn search_tight_alternating_tree(
    graph: &EdmondsGraph,
    duals: &DualState,
    matching: &MatchingState,
    root: usize,
) -> Result<TreeSearchResult, MatcherError> {
    let node_count = graph.adjacency.len();

    let mut label = vec![Label::Unlabeled; node_count];
    let mut parent: Vec<Option<usize>> = vec![None; node_count];
    let mut queue = VecDeque::new();

    // Start the search from the root vertex, labeling it as even and adding it to the queue.
    label[root] = Label::Even;
    queue.push_back(root);

    // Perform a breadth-first search to find an augmenting path, while also checking for tight edges and detecting blossoms.
    while let Some(u) = queue.pop_front() {
        for v in graph.try_tight_neighbors(duals, u)? {
            match label[v] {
                // If vertex `v` is unlabeled, label it as odd and label its mate as even,
                // then add the mate to the queue for further exploration.
                Label::Unlabeled => {
                    parent[v] = Some(u);

                    if matching.is_exposed(v) {
                        let path = try_reconstruct_path(root, v, &parent)?;

                        return Ok(TreeSearchResult {
                            tree: AlternatingTree { label, parent },
                            result: SearchResult::AugmentingPath(path),
                        });
                    }

                    let mate = matching
                        .mate
                        .get(v)
                        .copied()
                        .flatten()
                        .ok_or(MatcherError::MissingMate(v))?;

                    label[v] = Label::Odd;
                    label[mate] = Label::Even;
                    parent[mate] = Some(v);
                    queue.push_back(mate);
                }

                // If vertex `v` is labeled as even, it means that an edge between two even-labeled vertices has been found,
                // which indicates the presence of a blossom. The least common ancestor (LCA) of `u` and `v` is found in the search tree,
                // and the cycle formed by the blossom is reconstructed to return a `SearchResult::Blossom` containing the cycle,
                // the base of the blossom, and the edge that caused the blossom to be detected.
                Label::Even => {
                    if u == v {
                        continue;
                    }

                    if let Some(lca) = find_lca(u, v, &parent) {
                        let cycle = try_reconstruct_blossom_cycle(u, v, lca, &parent)?;

                        return Ok(TreeSearchResult {
                            tree: AlternatingTree { label, parent },
                            result: SearchResult::Blossom {
                                cycle,
                                base: lca,
                                edge: (u, v),
                            },
                        });
                    }
                }

                Label::Odd => {}
            }
        }
    }

    Ok(TreeSearchResult {
        tree: AlternatingTree { label, parent },
        result: SearchResult::None,
    })
}

/// Attempts to initialize the dual variables for the vertices in the graph based on the minimum edge weights,
/// ensuring that the initial dual values are set correctly to allow for the search for tight edges during
/// the execution of Edmonds' algorithm, and returning an error if any vertex has no neighbors or if any node index is invalid.
///
/// # Arguments
/// * `graph` - A reference to the graph represented as an `EdmondsGraph` for which to initialize the dual variables.
///
/// # Returns
/// * `Result<DualState, MatcherError>` - The initialized `DualState` containing the dual variables for the vertices in the graph,
///   or an error if any vertex has no neighbors or if any node index is invalid.
fn try_initialize_duals(graph: &EdmondsGraph) -> Result<DualState, MatcherError> {
    let mut duals = DualState::new(graph.adjacency.len());

    for node in 0..graph.adjacency.len() {
        let neighbors = graph
            .adjacency
            .get(node)
            .ok_or(MatcherError::InvalidNodeIndex(node, graph.adjacency.len()))?;

        let min_weight = neighbors
            .iter()
            .filter_map(|&neighbor| graph.weight(node, neighbor))
            .min()
            .ok_or(MatcherError::NoMatchingCandidate(node))?;

        duals.try_set(node, i64::from(min_weight))?;
    }

    Ok(duals)
}

/// Attempts to compute the minimum outgoing slack from the vertices labeled as even in the alternating tree,
/// which is necessary for determining how much to adjust the dual variables during the execution of Edmonds' algorithm,
/// and returns an error if any issues occur while computing the slack, such as invalid node indices or missing edge weights.
///
/// # Arguments
/// * `tree` - A reference to the `AlternatingTree` struct representing the current state of the alternating tree during the search for augmenting paths.
/// * `graph` - A reference to the graph represented as an `EdmondsGraph` for which to compute the minimum outgoing slack.
/// * `duals` - A reference to the current state of the dual variables for the vertices in the graph,
///   which is used to compute the slack for the edges during the search for augmenting paths.
///
/// # Returns
/// * `Result<i64, MatcherError>` - The minimum outgoing slack from the even-labeled vertices in the alternating tree,
///   or an error if any issues occur while computing the slack, such as invalid node indices or missing edge weights.
fn try_minimum_outgoing_slack(
    tree: &AlternatingTree,
    graph: &EdmondsGraph,
    duals: &DualState,
) -> Result<i64, MatcherError> {
    let mut minimum = i64::MAX;

    for u in tree.even_vertices() {
        for v in graph.neighbors(u) {
            match tree.label[v] {
                Label::Unlabeled => {
                    let slack = duals.try_slack(graph, u, v)?;

                    minimum = minimum.min(slack);
                }

                Label::Even | Label::Odd => {}
            }
        }
    }

    Ok(minimum)
}

/// Attempts to update the dual variables for the vertices in the alternating tree by adding a given delta to the even-labeled vertices
/// and subtracting the same delta from the odd-labeled vertices, which is necessary for maintaining the feasibility of the dual
/// variables during the execution of Edmonds' algorithm.
///
/// # Arguments
/// * `tree` - A reference to the `AlternatingTree` struct representing the current state of the alternating tree during the search for augmenting paths.
/// * `duals` - A mutable reference to the current state of the dual variables for the vertices in the graph,
///   which will be updated based on the labels of the vertices in the alternating tree.
/// * `delta` - The value to be added to the even-labeled vertices and subtracted from the odd-labeled vertices in the alternating
///   tree to maintain the feasibility of the dual variables during the execution of Edmonds' algorithm.
///
/// # Returns
/// * `Result<(), MatcherError>` - An empty result indicating that the dual variables were successfully updated, or an error if any issues occur while updating the dual variables, such as invalid node indices or attempts to set dual variables to invalid values.
fn try_update_duals(
    tree: &AlternatingTree,
    duals: &mut DualState,
    delta: i64,
) -> Result<(), MatcherError> {
    for node in tree.even_vertices() {
        duals.try_add(node, delta)?;
    }

    for node in tree.odd_vertices() {
        duals.try_add(node, -delta)?;
    }

    Ok(())
}

/// Builds a tight alternating tree starting from the given root vertex by performing a breadth-first search,
/// labeling vertices as even or odd based on their distance from the root, and returning the resulting
/// `AlternatingTree` struct containing the labels and parent pointers for the vertices in the tree,
/// or an error if any issues occur during the construction of the tree, such as missing mates or invalid node indices.
///
/// # Arguments
/// * `graph` - A reference to the graph represented as an `EdmondsGraph` for which to build the tight alternating tree.
/// * `duals` - A reference to the current state of the dual variables for the vertices in the graph,
///   which is used to determine tight edges during the construction of the tree.
/// * `matching` - A reference to the current state of the matching, which is used to determine the mates of vertices during the construction of the tree.
/// * `root` - The index of the root vertex from which to start building the tight alternating tree.
///
/// # Returns
/// * `Result<AlternatingTree, MatcherError>` - The resulting `AlternatingTree` struct containing the labels and parent pointers
///   for the vertices in the tree, or an error if any issues occur during the construction of the tree, such as missing mates or invalid node indices.
fn build_tight_alternating_tree(
    graph: &EdmondsGraph,
    duals: &DualState,
    matching: &MatchingState,
    root: usize,
) -> Result<AlternatingTree, MatcherError> {
    let node_count = graph.adjacency.len();

    let mut label = vec![Label::Unlabeled; node_count];
    let mut parent: Vec<Option<usize>> = vec![None; node_count];
    let mut queue = VecDeque::new();

    label[root] = Label::Even;
    queue.push_back(root);

    while let Some(u) = queue.pop_front() {
        for v in graph.try_tight_neighbors(duals, u)? {
            match label[v] {
                Label::Unlabeled => {
                    parent[v] = Some(u);

                    if matching.is_exposed(v) {
                        return Ok(AlternatingTree { label, parent });
                    }

                    let mate = matching
                        .mate
                        .get(v)
                        .copied()
                        .flatten()
                        .ok_or(MatcherError::MissingMate(v))?;

                    label[v] = Label::Odd;
                    label[mate] = Label::Even;
                    parent[mate] = Some(v);
                    queue.push_back(mate);
                }

                Label::Even => {}

                Label::Odd => {}
            }
        }
    }

    Ok(AlternatingTree { label, parent })
}

/// Attempts to find a weighted augmenting path in the graph using Edmonds' algorithm by repeatedly searching for tight alternating trees,
/// computing the minimum outgoing slack, and updating the dual variables until an augmenting path is found or it is determined that
/// no augmenting path exists, and returns an error if any issues occur during the search process, such as missing mates or invalid node indices.
///
/// # Arguments
/// * `graph` - A reference to the graph represented as an `EdmondsGraph` for which to find a weighted augmenting path.
/// * `duals` - A mutable reference to the current state of the dual variables for the vertices in the graph,
///   which will be updated during the search for a weighted augmenting path.
/// * `matching` - A reference to the current state of the matching, which is used to determine the mates
///   of vertices during the search for a weighted augmenting path.
/// * `root` - The index of the root vertex from which to start the search for a weighted augmenting path.
///
/// # Returns
/// * `Result<Option<Vec<usize>>, MatcherError>` - An optional vector of vertex indices representing the weighted augmenting path found,
///   or `None` if no augmenting path exists, or an error if any issues occur during the search process, such as missing mates or invalid node indices.
fn try_find_weighted_augmenting_path(
    graph: &EdmondsGraph,
    duals: &mut DualState,
    matching: &MatchingState,
    root: usize,
) -> Result<Option<Vec<usize>>, MatcherError> {
    loop {
        let search = search_tight_alternating_tree(graph, duals, matching, root)?;

        match search.result {
            SearchResult::AugmentingPath(path) => return Ok(Some(path)),
            SearchResult::Blossom { .. } => {
                return Err(MatcherError::BlossomExpansionNotImplemented);
            }
            SearchResult::None => {
                let tree = build_tight_alternating_tree(graph, duals, matching, root)?;
                let delta = try_minimum_outgoing_slack(&tree, graph, duals)?;
                if delta == i64::MAX {
                    return Ok(None);
                }

                try_update_duals(&tree, duals, delta)?;
            }
        }
    }
}

#[cfg(test)]
mod tests;
