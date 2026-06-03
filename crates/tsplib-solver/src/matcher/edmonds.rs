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

#[derive(Debug, PartialEq, Eq)]
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
    index_to_node_id: Vec<usize>,
    node_id_to_index: HashMap<usize, usize>,
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
        }

        Self {
            adjacency,
            index_to_node_id,
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
                Some(_) => Err(MatcherError::BlossomExpansionNotImplemented),
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

#[cfg(test)]
mod tests {
    use tsplib_core::models::{Edge, Node};

    use super::*;

    fn test_graph(adjacency: Vec<Vec<usize>>) -> EdmondsGraph {
        let nodes = (0..adjacency.len())
            .map(|id| Node {
                id,
                x: 0.0,
                y: 0.0,
                z: None,
            })
            .collect::<Vec<_>>();

        let mut edges = Vec::new();

        for u in 0..adjacency.len() {
            for &v in &adjacency[u] {
                if u < v {
                    edges.push(Edge { u, v, weight: 1 });
                }
            }
        }

        EdmondsGraph::from_graph(&Graph { nodes, edges })
    }

    #[test]
    fn augment_path_toggles_matching_edges() {
        let mut state = MatchingState::new(6);

        state.match_edge(1, 2);
        state.match_edge(3, 4);

        state.augment_path(&[0, 1, 2, 3, 4, 5]);

        assert_eq!(state.mate[0], Some(1));
        assert_eq!(state.mate[1], Some(0));

        assert_eq!(state.mate[2], Some(3));
        assert_eq!(state.mate[3], Some(2));

        assert_eq!(state.mate[4], Some(5));
        assert_eq!(state.mate[5], Some(4));
    }

    #[test]
    fn find_simple_augmenting_path_without_blossom() {
        let graph = test_graph(vec![
            vec![1],    // 0
            vec![0, 2], // 1
            vec![1, 3], // 2
            vec![2, 4], // 3
            vec![3, 5], // 4
            vec![4],    // 5
        ]);

        let mut matching = MatchingState::new(6);
        matching.match_edge(1, 2);
        matching.match_edge(3, 4);

        let path = search_alternating_tree(&graph, &matching, 0).expect("search should succeed");

        match path {
            SearchResult::AugmentingPath(vertices) => {
                assert_eq!(vertices, vec![0, 1, 2, 3, 4, 5]);

                matching.augment_path(&vertices);
            }
            _ => panic!("expected blossom"),
        }

        assert_eq!(matching.mate[0], Some(1));
        assert_eq!(matching.mate[1], Some(0));
        assert_eq!(matching.mate[2], Some(3));
        assert_eq!(matching.mate[3], Some(2));
        assert_eq!(matching.mate[4], Some(5));
        assert_eq!(matching.mate[5], Some(4));
    }

    #[test]
    fn find_lca_in_alternating_tree() {
        let parent = vec![
            None,    // 0 (root)
            Some(0), // 1
            Some(1), // 2
            Some(0), // 3
            Some(3), // 4
        ];

        let lca = find_lca(2, 4, &parent);

        assert_eq!(lca, Some(0));
    }

    #[test]
    fn reconstruct_blossom_cycle() {
        let parent = vec![
            None,    // 0 root/lca
            Some(0), // 1
            Some(1), // 2 = u
            Some(0), // 3
            Some(3), // 4 = v
        ];

        let cycle = try_reconstruct_blossom_cycle(2, 4, 0, &parent)
            .expect("should reconstruct blossom cycle");

        assert_eq!(cycle, vec![2, 1, 0, 3, 4]);
    }

    #[test]
    fn detect_simple_blossom_cycles() {
        let graph = test_graph(vec![
            vec![1, 3], // 0 root
            vec![0, 2], // 1
            vec![1, 4], // 2
            vec![0, 4], // 3
            vec![2, 3], // 4
        ]);

        let mut matching = MatchingState::new(5);
        matching.match_edge(1, 2);
        matching.match_edge(3, 4);

        let result = search_alternating_tree(&graph, &matching, 0).expect("search should succeed");

        match result {
            SearchResult::Blossom { cycle, base, edge } => {
                assert_eq!(cycle.len(), 5);
                assert_eq!(base, 0);
                assert!(matches!(edge, (2, 4) | (4, 2)));
                assert!(cycle.contains(&0));
                assert!(cycle.contains(&1));
                assert!(cycle.contains(&2));
                assert!(cycle.contains(&3));
                assert!(cycle.contains(&4));
            }
            _ => panic!("expected blossom"),
        }
    }

    #[test]
    fn shrink_graph_contracts_blossom_cycle() {
        let graph = test_graph(vec![
            vec![1, 3, 5], // 0
            vec![0, 2],    // 1
            vec![1, 4],    // 2
            vec![0, 4],    // 3
            vec![2, 3, 6], // 4
            vec![0],       // 5 external
            vec![4],       // 6 external
        ]);

        let blossom = Blossom::new(0, vec![2, 1, 0, 3, 4]);
        let shrunk = shrink_graph(&graph, &blossom);

        assert_eq!(shrunk.graph.adjacency.len(), 3);
        assert_eq!(shrunk.blossom_node, 2);

        assert_eq!(shrunk.original_to_shrunk[5], 0);
        assert_eq!(shrunk.original_to_shrunk[6], 1);

        for node in 0..=4 {
            assert_eq!(shrunk.original_to_shrunk[node], shrunk.blossom_node);
        }

        assert!(shrunk.graph.adjacency[shrunk.blossom_node].contains(&0));
        assert!(shrunk.graph.adjacency[shrunk.blossom_node].contains(&1));
        assert!(shrunk.graph.adjacency[0].contains(&shrunk.blossom_node));
        assert!(shrunk.graph.adjacency[1].contains(&shrunk.blossom_node));
    }

    #[test]
    fn shrink_matching_maps_external_matching_edge_to_blossom_node() {
        let graph = test_graph(vec![
            vec![1, 3, 5], // 0
            vec![0, 2],    // 1
            vec![1, 4],    // 2
            vec![0, 4],    // 3
            vec![2, 3, 6], // 4
            vec![0],       // 5 external
            vec![4],       // 6 external
        ]);

        let blossom = Blossom::new(0, vec![2, 1, 0, 3, 4]);
        let shrunk = shrink_graph(&graph, &blossom);

        let mut matching = MatchingState::new(7);
        matching.match_edge(1, 2); // inside blossom
        matching.match_edge(3, 4); // inside blossom
        matching.match_edge(0, 5); // blossom matched to outside

        let shrunk_matching = shrink_matching(&matching, &shrunk);

        let external_5 = shrunk.original_to_shrunk[5];

        assert_eq!(shrunk_matching.mate[shrunk.blossom_node], Some(external_5));
        assert_eq!(shrunk_matching.mate[external_5], Some(shrunk.blossom_node));
    }

    #[test]
    fn can_shrink_detected_blossom() {
        let graph = test_graph(vec![
            vec![1, 3], // 0 root / blossom base
            vec![0, 2], // 1
            vec![1, 4], // 2
            vec![0, 4], // 3
            vec![2, 3], // 4
        ]);

        let mut matching = MatchingState::new(5);
        matching.match_edge(1, 2);
        matching.match_edge(3, 4);

        let result = search_alternating_tree(&graph, &matching, 0).expect("search should succeed");

        let SearchResult::Blossom { cycle, base, .. } = result else {
            panic!("expected blossom");
        };

        let blossom = Blossom::new(base, cycle);
        let shrunk = shrink_graph(&graph, &blossom);
        let shrunk_matching = shrink_matching(&matching, &shrunk);

        assert_eq!(blossom.base, 0);
        assert_eq!(blossom.cycle.len(), 5);

        assert_eq!(shrunk.graph.adjacency.len(), 1);
        assert_eq!(shrunk.blossom_node, 0);

        for node in 0..5 {
            assert_eq!(shrunk.original_to_shrunk[node], shrunk.blossom_node);
        }

        assert!(shrunk.graph.adjacency[shrunk.blossom_node].is_empty());
        assert!(shrunk_matching.mate[shrunk.blossom_node].is_none());
    }

    #[test]
    fn edmonds_finds_augmenting_path_without_blossom() {
        let graph = test_graph(vec![
            vec![1],    // 0
            vec![0, 2], // 1
            vec![1, 3], // 2
            vec![2, 4], // 3
            vec![3, 5], // 4
            vec![4],    // 5
        ]);

        let mut matching = MatchingState::new(6);
        matching.match_edge(1, 2);
        matching.match_edge(3, 4);

        let path = try_find_augmenting_path_edmonds(&graph, &matching, 0)
            .expect("search should succeed")
            .expect("augmenting path should exist");

        assert_eq!(path, vec![0, 1, 2, 3, 4, 5]);
    }

    #[test]
    fn edmonds_detects_blossom_but_expansion_is_not_implemented() {
        let graph = test_graph(vec![
            vec![1, 3, 5], // 0 base
            vec![0, 2],    // 1
            vec![1, 4],    // 2
            vec![0, 4],    // 3
            vec![2, 3],    // 4
            vec![0, 6],    // 5
            vec![5],       // 6 exposed
        ]);

        let mut matching = MatchingState::new(7);
        matching.match_edge(1, 2);
        matching.match_edge(3, 4);
        matching.match_edge(0, 5);

        let result = search_alternating_tree(&graph, &matching, 6).expect("search should succeed");

        let SearchResult::Blossom { cycle, base, .. } = result else {
            panic!("expected blossom before shrinking");
        };

        let blossom = Blossom::new(base, cycle);
        let shrunk = shrink_graph(&graph, &blossom);
        let shrunk_matching = shrink_matching(&matching, &shrunk);
        let shrunk_root = shrunk.original_to_shrunk[6];

        let shrunk_result = search_alternating_tree(&shrunk.graph, &shrunk_matching, shrunk_root)
            .expect("shrunk search should succeed");

        assert!(matches!(shrunk_result, SearchResult::None));
    }

    #[test]
    fn blossom_cycle_index() {
        let blossom = Blossom::new(0, vec![2, 1, 0, 3, 4]);

        assert_eq!(blossom.cycle_index(2), Some(0));
        assert_eq!(blossom.cycle_index(0), Some(2));
        assert_eq!(blossom.cycle_index(4), Some(4));
        assert_eq!(blossom.cycle_index(42), None);
    }

    #[test]
    fn blossom_cycle_paths_between() {
        let blossom = Blossom::new(0, vec![2, 1, 0, 3, 4]);

        let (forward, backward) = blossom
            .cycle_paths_between(2, 4)
            .expect("path should exist");

        assert_eq!(forward, vec![2, 1, 0, 3, 4]);
        assert_eq!(backward, vec![2, 4]);
    }

    #[test]
    fn detects_alternating_path() {
        let mut matching = MatchingState::new(5);
        matching.match_edge(1, 2);
        matching.match_edge(3, 4);

        assert!(is_alternating_path(&[0, 1, 2, 3, 4], &matching));
        assert!(!is_alternating_path(&[0, 1, 3, 4], &matching));
    }

    #[test]
    fn chooses_alternating_blossom_path() {
        let blossom = Blossom::new(0, vec![2, 1, 0, 3, 4]);

        let mut matching = MatchingState::new(5);
        matching.match_edge(1, 2);
        matching.match_edge(3, 4);

        let path = try_choose_alternating_blossom_path(&blossom, 0, 4, &matching)
            .expect("should choose alternating blossom path");

        assert_eq!(path, vec![0, 3, 4]);
    }
}
