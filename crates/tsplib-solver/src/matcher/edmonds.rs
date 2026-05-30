//! This module implements the Edmonds' Blossom algorithm for finding a minimum weight perfect matching in a graph.
use std::collections::{HashSet, VecDeque};

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
        edge: (usize, usize),
    },
    None,
}

/// A struct to represent the state of the matching during the algorithm.
#[derive(Debug, Clone)]
struct MatchingState {
    /// The `mate` vector holds the index of the matched vertex for each vertex in the graph.
    /// If a vertex is unmatched, its entry is `None`.
    mate: Vec<Option<usize>>,
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

/// Creates an iterator over the neighbors of a given node in the graph.
///
/// # Arguments
/// * `graph` - A reference to the adjacency list representation of the graph.
/// * `node` - The index of the node whose neighbors are to be returned.
///
/// # Returns
/// * `impl Iterator<Item = usize>` - An iterator over the indices of the neighboring vertices.
fn neighbors(graph: &[Vec<usize>], node: usize) -> impl Iterator<Item = usize> + '_ {
    graph[node].iter().copied()
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
/// * `graph` - A reference to the adjacency list representation of the graph.
/// * `matching` - A reference to the current state of the matching.
/// * `root` - The index of the root vertex from which to start the search for an augmenting path.
///
/// # Returns
/// * `Option<Vec<usize>>` - An optional vector of vertex indices representing the augmenting path found.
///   If no augmenting path exists, returns `None`.
fn search_alternating_tree(
    graph: &[Vec<usize>],
    matching: &MatchingState,
    root: usize,
) -> Result<SearchResult, MatcherError> {
    let node_count = graph.len();

    // Initialize the label and parent vectors for the breadth-first search.
    let mut label = vec![Label::Unlabeled; node_count];
    let mut parent: Vec<Option<usize>> = vec![None; node_count];
    let mut queue = VecDeque::new();

    // Start the search from the root vertex, labeling it as even and adding it to the queue.
    label[root] = Label::Even;
    queue.push_back(root);

    // Perform a breadth-first search to find an augmenting path.
    while let Some(u) = queue.pop_front() {
        for v in neighbors(graph, u) {
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

#[cfg(test)]
mod tests {
    use super::*;

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
        let graph = vec![
            vec![1],    // 0
            vec![0, 2], // 1
            vec![1, 3], // 2
            vec![2, 4], // 3
            vec![3, 5], // 4
            vec![4],    // 5
        ];

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
        let graph = vec![
            vec![1, 3], // 0 root
            vec![0, 2], // 1
            vec![1, 4], // 2
            vec![0, 4], // 3
            vec![2, 3], // 4
        ];

        let mut matching = MatchingState::new(5);
        matching.match_edge(1, 2);
        matching.match_edge(3, 4);

        let result = search_alternating_tree(&graph, &matching, 0).expect("search should succeed");

        match result {
            SearchResult::Blossom { cycle, edge } => {
                assert_eq!(cycle.len(), 5);
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
}
