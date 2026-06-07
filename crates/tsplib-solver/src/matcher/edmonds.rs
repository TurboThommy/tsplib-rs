//! This module implements the Edmonds' Blossom algorithm for finding a minimum weight perfect matching
//! over the complete graph induced by a given set of vertices (the odd-degree MST vertices in Christofides).
use std::{
    collections::{HashMap, HashSet},
    mem::swap,
};

use tsplib_core::models::{Edge, Graph, TsplibInstance};

use crate::{PerfectMatchingAlgorithm, errors::MatcherError};

/// Factor by which original edge weights are multiplied so that otherwise half-integral dual updates stay integral.
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

/// The mutable derived graph `G'` together with all per-node algorithm state.
struct DerivedGraph<'a> {
    base: &'a BaseGraph,

    /// Number of original nodes
    n0: usize,

    /// Dual value y_v for every node id (original or pseudonode).
    duals: Vec<i64>,

    /// Immediate enclosing pseudonode of each node or `None` if outermost
    container: Vec<Option<usize>>,

    /// whether the node is currently an outermost node of the derived graph
    outer: Vec<bool>,

    /// For each pseudonode, the cyclic list of member ids (base first)
    cycle_of: HashMap<usize, Vec<usize>>,

    /// For consecutive cycle members (canonical key) the original edge that realized their (tight) cycle edge when the blossom formed
    ring_edge: HashMap<(usize, usize), (usize, usize)>,

    /// Alternating-tree label for every node id.
    label: Vec<Label>,

    /// Tree parent pointer (in `G'`) for every node id; `None` for roots/free.
    parent: Vec<Option<usize>>,

    /// original edge realizing the tree edge to the parent `(o_in_parent, o_in_self)`
    parent_edge: Vec<Option<(usize, usize)>>,

    /// Matching over derived-node ids: `mate[v] == Some(w)` iff `vw` is in `M'`.
    mate: Vec<Option<usize>>,

    /// original edge realizing the match `(o_in_self, o_in_partner)`
    mate_edge: Vec<Option<(usize, usize)>>,
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

/// Action selected by the search at each step
enum Action {
    /// An augmenting path was found from an even node `u` to an exposed node `v` over edge `e`.
    Augment(usize, usize, (usize, usize)),

    /// An extension of the alternating tree was found from an even node `u` to an odd node `v` over edge `e`.
    Extend(usize, usize, (usize, usize)),

    /// A tight edge was found joining two even nodes `u` and `v`, indicating a blossom to shrink over edge `e`.
    Shrink(usize, usize, (usize, usize)),

    /// An odd pseudonode `v` with zero dual value was found, indicating a pseudonode to expand.
    Expand(usize),

    /// No equality-edge step applies, but the tree is not frustrated, so a dual change could create new tight edges.
    DualChange,
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

        let graph = try_build_complete_graph_for_vertices(&odd_vertices, problem)?;
        let base = BaseGraph::from_graph(&graph);

        let mut derived = DerivedGraph::new(&base);
        derived.try_find_matching()?;

        base.try_matching_to_edges(derived.original_mate())
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
        // Build a mapping from original node indices to their TSPLIB node IDs and vice versa
        let index_to_node_id = graph.nodes.iter().map(|node| node.id).collect::<Vec<_>>();
        let node_id_to_index = index_to_node_id
            .iter()
            .enumerate()
            .map(|(index, &node_id)| (node_id, index))
            .collect::<HashMap<_, _>>();

        let mut weights = HashMap::new();

        // Iterate over the edges in the graph and populate the weights HashMap with scaled weights for each edge.
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
    fn try_node_id(&self, index: usize) -> Result<usize, MatcherError> {
        self.index_to_node_id
            .get(index)
            .copied()
            .ok_or(MatcherError::InvalidNodeIndex(
                index,
                self.index_to_node_id.len(),
            ))
    }

    /// Converts a final mate vector over original nodes into TSPLIB edges undoing the scaling.
    ///
    /// # Arguments
    /// * `mate` - A slice where `mate[u]` gives the index of the vertex matched to vertex `u`, or `None` if `u` is unmatched.
    /// * `self` - The `BaseGraph` instance containing the mapping from indices to node IDs and the edge weights.
    ///
    /// # Returns
    /// * `Result<Vec<Edge>, MatcherError>` - A result containing a vector of `Edge` instances representing the matched edges
    ///   in terms of TSPLIB node IDs and original weights, or an error if the mate vector is invalid or if any edge is missing.
    fn try_matching_to_edges(&self, mate: &[Option<usize>]) -> Result<Vec<Edge>, MatcherError> {
        if mate.len() < self.node_count {
            return Err(MatcherError::Internal(
                "Mate vector is less than node count",
            ));
        }

        let mut edges = Vec::with_capacity(self.node_count / 2);

        // Iterate over the mate vector to construct the list of matched edges.
        // Each edge is emitted once by only processing pairs where `u < v`.
        for (u, v) in mate.iter().enumerate().take(self.node_count) {
            let Some(v) = *v else {
                return Err(MatcherError::NodeUnmatched(u));
            };

            // A correct lift leaves every original node matched to another
            if v >= self.node_count {
                return Err(MatcherError::MateNotLifted(u, v));
            }

            // Emit each edge once: skip the mirror copy where u >= v
            if u >= v {
                continue;
            }

            let scaled = self.weight(u, v).ok_or(MatcherError::MissingEdge(u, v))?;

            // The original weight must be an integer, so the scaled weight must be divisible by WEIGHT_SCALE
            edges.push(Edge {
                u: self.try_node_id(u)?,
                v: self.try_node_id(v)?,
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

        Self {
            base,
            n0: n,
            duals: vec![0; n],
            container: vec![None; n],
            outer: vec![true; n],
            cycle_of: HashMap::new(),
            ring_edge: HashMap::new(),
            label: vec![Label::Free; n],
            parent: vec![None; n],
            parent_edge: vec![None; n],
            mate: vec![None; n],
            mate_edge: vec![None; n],
        }
    }

    /// Checks if a given vertex in the derived graph is a pseudonode (blossom) or an original vertex.
    ///
    /// # Arguments
    /// * `v` - The index of the vertex in the derived graph to check.
    ///
    /// # Returns
    /// * `bool` - `true` if the vertex is a pseudonode (blossom), `false` if it is an original vertex.
    fn is_pseudonode(&self, v: usize) -> bool {
        v >= self.n0
    }

    /// Returns the total number of nodes (original and pseudonodes) currently in the derived graph.
    ///
    /// # Returns
    /// * `usize` - The total number of nodes in the derived graph, including both original vertices and pseudonodes (blossoms).
    fn total_nodes(&self) -> usize {
        self.duals.len()
    }

    /// Get the outermost node currently containing node `i`.
    ///
    /// # Arguments
    /// * `i` - The index of the node for which to find the outermost containing node.
    ///
    /// # Returns
    /// * `Result<usize, MatcherError>` - The index of the outermost node currently containing node `i` in the derived graph,
    ///   or an error if the containment structure is invalid.
    fn try_outermost(&self, i: usize) -> Result<usize, MatcherError> {
        let mut cur = i;

        // Follow the container chain up to the outermost node. The invariant is that every non-outer node must have a container.
        while !self.outer[cur] {
            cur = self.container[cur].ok_or(MatcherError::Internal(
                "Non-outer node must have a container",
            ))?;
        }

        Ok(cur)
    }

    /// Sum of duals along the containment chain from `i` up to its outermost node.
    ///
    /// # Arguments
    /// * `i` - The index of the node for which to compute the sum of duals along the containment chain.
    ///
    /// # Returns
    /// * `Result<i64, MatcherError>` - The sum of the dual values for all nodes along the containment chain from node `i`
    ///   up to its outermost node in the derived graph, or an error if the containment structure is invalid.
    fn try_ystar(&self, i: usize) -> Result<i64, MatcherError> {
        let mut sum = 0;
        let mut cur = i;

        // Follow the container chain up to the outermost node, summing the dual values.
        loop {
            sum += self.duals[cur];
            if self.outer[cur] {
                break;
            }

            // The invariant is that every non-outer node must have a container.
            cur = self.container[cur].ok_or(MatcherError::Internal(
                "Non-outer node must have a container",
            ))?;
        }

        Ok(sum)
    }

    /// Slack of an original edge `c_ij - Ystar(i) - Ystar(j)`.
    ///
    /// # Arguments
    /// * `i` - The index of the first vertex in the base graph.
    /// * `j` - The index of the second vertex in the base graph.
    ///
    /// # Returns
    /// * `Result<i64, MatcherError>` - The slack of the original edge between vertices `i` and `j`, calculated as the scaled weight of the edge
    ///   minus the sum of the dual values along the containment chains of `i` and `j`. Returns an error if the edge does not exist in the base graph
    ///   or if the containment structure is invalid.
    fn try_slack_orig(&self, i: usize, j: usize) -> Result<i64, MatcherError> {
        // The original edge must exist in the complete graph
        let w = self.base.weight(i, j).ok_or(MatcherError::Internal(
            "Complete graph: original edge must exist",
        ))?;

        let slack = w - self.try_ystar(i)? - self.try_ystar(j)?;

        Ok(slack)
    }

    /// Returns a vector of the indices of the currently outermost nodes in the derived graph.
    ///
    /// # Returns
    /// * `Vec<usize>` - A vector containing the indices of the currently outermost nodes in the derived graph.
    fn outer_nodes(&self) -> Vec<usize> {
        (0..self.total_nodes()).filter(|&v| self.outer[v]).collect()
    }

    /// Finds a tight edge between the outermost nodes currently containing `u` and `v`, if one exists.
    ///
    /// # Arguments
    /// * `u` - First outermost derived node.
    /// * `v` - Second outermost derived node.
    ///
    /// # Returns
    /// * `Result<Option<(usize, usize)>, MatcherError>` - An option containing a tuple of the indices of the outermost nodes in the derived graph
    ///   that currently contain `u` and `v`, respectively, if a tight edge exists between them; otherwise, `None`.
    ///   Returns an error if the containment structure is invalid.
    fn try_tight_between(
        &self,
        u: usize,
        v: usize,
    ) -> Result<Option<(usize, usize)>, MatcherError> {
        // A tight edge between the outermost nodes containing u and v must be realized by an original edge between some i
        // in the outermost node containing u and some j in the outermost node containing v.
        for i in 0..self.n0 {
            if self.try_outermost(i)? != u {
                continue;
            }

            for j in 0..self.n0 {
                if i == j || self.try_outermost(j)? != v {
                    continue;
                }

                // The invariant is that the slack of an original edge between the outermost nodes containing u and v
                // is zero iff it is a tight edge in the derived graph between those nodes.
                if self.try_slack_orig(i, j)? == 0 {
                    return Ok(Some((i, j)));
                }
            }
        }
        Ok(None)
    }

    /// Finds the minimum slack of any edge between the outermost nodes currently containing `u` and `v`.
    ///
    /// # Arguments
    /// * `u` - The index of the first vertex in the base graph.
    /// * `v` - The index of the second vertex in the base graph.
    ///
    /// # Returns
    /// * `Result<Option<i64>, MatcherError>` - An option containing the minimum slack of any edge between
    ///   the outermost nodes currently containing `u` and `v`, or `None` if there are no edges between them.
    ///   Returns an error if the containment structure is invalid.
    fn try_min_slack(&self, u: usize, v: usize) -> Result<Option<i64>, MatcherError> {
        let mut best: Option<i64> = None;

        for i in 0..self.n0 {
            if self.try_outermost(i)? != u {
                continue;
            }

            for j in 0..self.n0 {
                if i == j || self.try_outermost(j)? != v {
                    continue;
                }

                let s = self.try_slack_orig(i, j)?;
                best = Some(match best {
                    Some(b) => b.min(s),
                    None => s,
                });
            }
        }

        Ok(best)
    }

    /// Finds any edge between the outermost nodes currently containing `u` and `v`, if one exists.
    ///
    /// # Arguments
    /// * `u` - The index of the first vertex in the base graph.
    /// * `v` - The index of the second vertex in the base graph.
    ///
    /// # Returns
    /// * `Result<Option<(usize, usize)>, MatcherError>` - An option containing a tuple of the indices of the outermost nodes in the derived graph
    ///   that currently contain `u` and `v`, respectively, if any edge exists between them; otherwise, `None`.
    ///   Returns an error if the containment structure is invalid.
    fn try_any_edge(&self, u: usize, v: usize) -> Result<Option<(usize, usize)>, MatcherError> {
        for i in 0..self.n0 {
            if self.try_outermost(i)? != u {
                continue;
            }

            for j in 0..self.n0 {
                if self.try_outermost(j)? == v && i != j {
                    return Ok(Some((i, j)));
                }
            }
        }

        Ok(None)
    }

    /// Drives the algorithm to a perfect matching of the derived graph, leaving the result in `self.mate`.
    /// For each exposed node an alternating tree is grown, performing dual changes as needed until the node is
    /// matched by an augmenting path.
    ///
    /// # Returns
    /// * `Result<(), MatcherError>` - A result indicating success or failure of the matching process.
    fn try_find_matching(&mut self) -> Result<(), MatcherError> {
        loop {
            let root = (0..self.total_nodes()).find(|&v| self.outer[v] && self.mate[v].is_none());
            let Some(root) = root else {
                return self.try_extract_matching();
            };

            self.clear_tree();
            self.label[root] = Label::Even;
            self.try_grow(root)?;
        }
    }

    /// Resets the alternating tree state by clearing all labels and parent pointers in the derived graph.
    fn clear_tree(&mut self) {
        for v in 0..self.total_nodes() {
            self.label[v] = Label::Free;
            self.parent[v] = None;
            self.parent_edge[v] = None;
        }
    }

    /// Grows the alternating tree from a given root vertex over tight edges in the derived graph,
    /// looking for an augmenting path, a blossom to shrink, or an odd pseudonode to expand.
    ///
    /// # Arguments
    /// * `root` - The index of the root vertex from which to grow the alternating tree. This vertex must be active and exposed.
    ///
    /// # Returns
    /// * `Result<(), MatcherError>` - A result indicating success or failure of the tree growth process. Returns `Ok(())`
    ///   if an augmenting path was found and the matching was enlarged, or if a blossom was shrunk or a pseudonode was expanded successfully.
    ///   Returns an error if any step of the process fails due to invalid state or if no solution can be found.
    fn try_grow(&mut self, root: usize) -> Result<(), MatcherError> {
        loop {
            match self.try_find_action()? {
                Action::Augment(u, v, e) => {
                    self.try_augment(u, v, e)?;
                    return Ok(());
                }

                Action::Extend(u, v, e) => {
                    // Add the edge uv to the tree, making v odd and the mate of v even.
                    let m = self.mate[v].ok_or(MatcherError::MissingMate(v))?;
                    self.label[v] = Label::Odd;
                    self.parent[v] = Some(u);
                    self.parent_edge[v] = Some(e);
                    self.label[m] = Label::Even;
                    self.parent[m] = Some(v);
                    self.parent_edge[m] = self.mate_edge[m];
                }

                Action::Shrink(u, v, e) => {
                    self.try_shrink(u, v, e)?;

                    // root is even; the new pseudonode keeps the tree rooted at the same exposed node
                    let _ = root;
                }

                Action::Expand(v) => self.try_expand(v)?,

                Action::DualChange => self.try_dual_change()?,
            }
        }
    }

    /// Finds the next action to take based on the current state of the alternating tree and the derived graph.
    ///
    /// # Returns
    /// * `Result<Action, MatcherError>` - A result containing the next action to take, which can be one of the following:
    ///   - `Action::Augment(u, v, e)`: An augmenting path was found from an even node `u` to an exposed node `v` over edge `e`.
    ///   - `Action::Extend(u, v, e)`: An extension of the alternating tree was found from an even node `u` to an odd node `v` over edge `e`.
    ///   - `Action::Shrink(u, v, e)`: A tight edge was found joining two even nodes `u` and `v`, indicating a blossom to shrink over edge `e`.
    ///   - `Action::Expand(v)`: An odd pseudonode `v` with zero dual value was found, indicating a pseudonode to expand.
    ///   - `Action::DualChange`: No equality-edge step applies, but the tree is not frustrated, so a dual change could create new tight edges.
    fn try_find_action(&self) -> Result<Action, MatcherError> {
        let outers = self.outer_nodes();
        let mut shrink_candidate: Option<(usize, usize, (usize, usize))> = None;

        // Check for an augmenting path or an extension first by iterating over pairs of outermost nodes in the derived graph
        // and looking for tight edges between them.
        for &u in &outers {
            if self.label[u] != Label::Even {
                continue;
            }

            for &v in &outers {
                if v == u {
                    continue;
                }

                let Some(e) = self.try_tight_between(u, v)? else {
                    continue;
                };

                match self.label[v] {
                    // A tight edge to a free vertex means we found an augmenting path from the root to an exposed vertex,
                    // so we can stop and return this action.
                    Label::Free => {
                        if self.mate[v].is_none() {
                            return Ok(Action::Augment(u, v, e));
                        } else {
                            return Ok(Action::Extend(u, v, e));
                        }
                    }

                    // A tight edge to another even vertex in the same tree means we found a blossom to shrink,
                    // but we have to keep looking for an augmenting path or extension in case there is one that takes priority over shrinking.
                    Label::Even => {
                        if shrink_candidate.is_none() && self.same_tree(u, v) {
                            shrink_candidate = Some((u, v, e));
                        }
                    }

                    // A tight edge to an odd vertex means we found a path that doesn't lead to an augmenting path or extension,
                    // so we can ignore it.
                    Label::Odd => {}
                }
            }
        }

        // If we didn't find an augmenting path or extension, but we found a shrink candidate, return it.
        if let Some((u, v, e)) = shrink_candidate {
            return Ok(Action::Shrink(u, v, e));
        }

        // If we didn't find any tight edges between outermost nodes, check for an odd pseudonode with zero dual value to expand.
        for &u in &outers {
            if self.label[u] == Label::Odd && self.is_pseudonode(u) && self.duals[u] == 0 {
                return Ok(Action::Expand(u));
            }
        }

        // If we didn't find any of the above, we have to do a dual change to create new tight edges.
        Ok(Action::DualChange)
    }

    /// Checks if two vertices in the derived graph belong to the same alternating tree.
    ///
    /// # Arguments
    /// * `a` - The index of the first vertex in the derived graph.
    /// * `b` - The index of the second vertex in the derived graph.
    ///
    /// # Returns
    /// * `bool` - `true` if vertices `a` and `b` belong to the same alternating tree in the derived graph, `false` otherwise.
    fn same_tree(&self, a: usize, b: usize) -> bool {
        let mut ra = a;
        while let Some(p) = self.parent[ra] {
            ra = p;
        }

        let mut rb = b;
        while let Some(p) = self.parent[rb] {
            rb = p;
        }

        ra == rb
    }

    /// Augments the matching along the path from vertex `u` to vertex `v` in the derived graph, using edge `e` to connect them.
    ///
    /// # Arguments
    /// * `u` - The index of the starting vertex in the derived graph, which is an even node in the alternating tree.
    /// * `v` - The index of the ending vertex in the derived graph, which is an exposed node in the alternating tree.
    /// * `e` - A tuple representing the original edge (i, j) in the base graph that connects the outermost nodes containing `u` and `v`.
    ///
    /// # Returns
    /// * `Result<(), MatcherError>` - A result indicating success or failure of the augmentation process.
    ///   Returns `Ok(())` if the augmentation was successful, or an error if the augmenting path is invalid or if any edge in the path is missing.
    fn try_augment(&mut self, u: usize, v: usize, e: (usize, usize)) -> Result<(), MatcherError> {
        // Path u, parent(u), ..., root
        let mut path = Vec::new();
        let mut node = Some(u);

        // Follow the parent pointers from u up to the root, recording the path of vertices in the derived graph.
        while let Some(x) = node {
            path.push(x);
            node = self.parent[x];
        }

        self.try_set_match(u, v, e)?;

        let mut i = 1;
        while i + 1 < path.len() {
            let a = path[i];
            let b = path[i + 1];

            // The path is guaranteed to be valid, so there must be a tight edge between the outermost nodes containing a and b.
            let e2 = self
                .try_tight_between(a, b)?
                .ok_or(MatcherError::InvalidAugmentingPath)?;

            // Set the match between a and b using the tight edge e2 that connects them,
            // which must exist since they are currently matched to each other.
            self.try_set_match(a, b, e2)?;
            i += 2;
        }
        Ok(())
    }

    /// Sets the match between two vertices in the derived graph and records the original edge that realizes this match.
    ///
    /// # Arguments
    /// * `a` - The index of the first vertex in the derived graph to be matched.
    /// * `b` - The index of the second vertex in the derived graph to be matched with the first vertex.
    /// * `e` - A tuple representing the original edge (i, j) in the base graph that connects the outermost nodes containing `a` and `b`.
    ///
    /// # Returns
    /// * `Result<(), MatcherError>` - A result indicating success or failure of setting the match.
    ///   Returns `Ok(())` if the match was set successfully, or an error if the outermost nodes containing `a` and `b`
    ///   do not match the expected vertices in edge `e`.
    fn try_set_match(&mut self, a: usize, b: usize, e: (usize, usize)) -> Result<(), MatcherError> {
        let (mut oa, mut ob) = e;
        if self.try_outermost(oa)? != a {
            swap(&mut oa, &mut ob);
        }
        self.mate[a] = Some(b);
        self.mate[b] = Some(a);
        self.mate_edge[a] = Some((oa, ob));
        self.mate_edge[b] = Some((ob, oa));

        Ok(())
    }

    /// Performs a dual change.
    ///
    /// # Returns
    /// * `Result<(), MatcherError>` - A result indicating success or failure of the dual change process.
    ///   Returns `Ok(())` if the dual change was successful, or an error if no solution can be found for the matching problem.
    fn try_dual_change(&mut self) -> Result<(), MatcherError> {
        let outers = self.outer_nodes();
        let mut eps: Option<i64> = None;

        // Consider all pairs of outermost nodes to find the minimum epsilon that would create new tight edges between them
        //  by decreasing the duals of even nodes and/or increasing the duals of odd nodes.
        for &u in &outers {
            // Only consider pairs of outermost nodes where u is even:
            // Odd nodes get their duals increased in the dual change step,
            // so they don't contribute to the minimum epsilon that would create new tight edges.
            if self.label[u] != Label::Even {
                continue;
            }

            // For each pair of outermost nodes, find the minimum slack of any edge between them, since that determines how much
            // the duals can be changed before a new tight edge appears.
            for &v in &outers {
                if v == u {
                    continue;
                }

                // Only consider pairs of outermost nodes where u is even, since those are the only ones
                // that can lead to new tight edges by decreasing their duals and/or increasing the duals of odd nodes.
                let Some(s) = self.try_min_slack(u, v)? else {
                    continue;
                };

                match self.label[v] {
                    // eps1: a tight edge from this even node to a free node
                    // outside the tree appears once eps reaches this slack.
                    Label::Free => eps = Some(take_min(eps, s)),

                    // eps2: between two even nodes both duals rise by eps, so the
                    // edge tightens at slack/2. `u < v` counts each pair once.
                    Label::Even => {
                        if u < v {
                            debug_assert!(s % 2 == 0, "odd B(T)->B(T) slack under x2 scaling");
                            eps = Some(take_min(eps, s / 2));
                        }
                    }

                    // Odd nodes get their duals increased in the dual change step,
                    // so they don't contribute to the minimum epsilon that would create new tight edges.
                    Label::Odd => {}
                }
            }
        }

        // Consider the duals of odd pseudonodes, since increasing their duals in the dual change step could
        // also create new tight edges to them from even nodes in the same tree.
        for &u in &outers {
            if self.label[u] == Label::Odd && self.is_pseudonode(u) {
                eps = Some(take_min(eps, self.duals[u]));
            }
        }

        // Update the duals of all nodes according to their labels. Even nodes get their duals decreased,
        // odd nodes get their duals increased and free nodes are unaffected.
        let eps = eps.ok_or(MatcherError::NoSolution)?;
        for &u in &outers {
            match self.label[u] {
                Label::Even => self.duals[u] += eps,
                Label::Odd => self.duals[u] -= eps,
                Label::Free => {}
            }
        }

        Ok(())
    }

    /// Finds the cycle formed by the tight edge between two even nodes `u` and `v` in the same tree, which indicates a blossom to shrink.
    ///
    /// # Arguments
    /// * `u` - The index of the first even node in the derived graph.
    /// * `v` - The index of the second even node in the derived graph.
    ///
    /// # Returns
    /// * `Result<(Vec<usize>, usize), MatcherError>` - A result containing a tuple with the vector of node indices
    ///   forming the cycle in the derived graph and the index of the least common ancestor (LCA) of `u` and `v` in the alternating tree,
    ///   or an error if the cycle cannot be found due to invalid state.
    fn try_find_cycle(&self, u: usize, v: usize) -> Result<(Vec<usize>, usize), MatcherError> {
        // The blossom cycle is: LCA, then u's branch reversed, then v's branch; the
        // closing edge between the two tips is the tight edge uv that triggered the
        // shrink. Consecutive entries are thus tree- or blossom-adjacent.
        let mut pu = Vec::new();
        let mut cur = Some(u);

        // Follow the parent pointers from u up to the root, recording the path of vertices in the derived graph.
        while let Some(x) = cur {
            pu.push(x);
            cur = self.parent[x];
        }

        let idx: HashMap<usize, usize> = pu.iter().enumerate().map(|(i, &x)| (x, i)).collect();
        let mut pv = Vec::new();
        let mut cur = Some(v);

        // Follow the parent pointers from v up to the root, recording the path of vertices in the derived graph until a common ancestor with u is found.
        let lca = loop {
            match cur {
                // If a vertex that is not on the u-path is encountered,
                // add it to the v-path and continue following the parent pointers.
                Some(x) if !idx.contains_key(&x) => {
                    pv.push(x);
                    cur = self.parent[x];
                }

                // If a vertex on the u-path is encountered, least common ancestor (LCA) of u and v has in the alternating tree has been found
                Some(x) => break x,

                // If the root of the tree is reached without finding a common ancestor,
                // then the state is invalid since u and v are supposed to be in the same tree, so return an error.
                None => return Err(MatcherError::PathReconstructionError),
            }
        };

        // The cycle is formed by the path from u to the LCA, followed by the path from v to the LCA in reverse.
        // The LCA is included only once at the start of the cycle.
        let lca_pos = *idx
            .get(&lca)
            .ok_or(MatcherError::Internal("LCA not on u-path."))?;

        let mut cycle = Vec::with_capacity(lca_pos + pv.len() + 1);
        cycle.push(lca);

        // Add the path from u to the LCA in reverse order, so that the cycle is oriented
        //  consistently with the direction of the edges from u and v to the LCA.
        for &x in pu[..lca_pos].iter().rev() {
            cycle.push(x);
        }
        cycle.extend(pv);

        Ok((cycle, lca))
    }

    /// Shrinks the blossom formed by the tight edge between two even nodes `u` and `v` in the same tree,
    /// creating a new pseudonode in the derived graph.
    ///
    /// # Arguments
    /// * `u` - The index of the first even node in the derived graph that forms the blossom.
    /// * `v` - The index of the second even node in the derived graph that forms the blossom.
    /// * `e` - A tuple representing the original edge (i, j) in the base graph that connects the outermost nodes containing `u` and `v`,
    ///   which is the tight edge that indicates the presence of the blossom.
    ///
    /// # Returns
    /// * `Result<usize, MatcherError>` - A result containing the index of the newly created pseudonode in the derived graph
    ///   that represents the shrunk blossom, or an error if the shrinking process fails due to invalid state or if the cycle cannot be found.
    fn try_shrink(&mut self, u: usize, v: usize, e: (usize, usize)) -> Result<usize, MatcherError> {
        // Reconstruct the blossom cycle and its base (the LCA) from the tree.
        let (cycle, base) = self.try_find_cycle(u, v)?;

        let c = self.total_nodes();
        let n = cycle.len();

        let mut ring: Vec<Option<(usize, usize)>> = Vec::with_capacity(n);
        for k in 0..n {
            let a = cycle[k];
            let b = cycle[(k + 1) % n];

            // Skip repeated vertices as they don't correspond to any edge in the base graph and would add a self-loop to the ring.
            if a == b {
                ring.push(None);
                continue;
            }

            // The tight edge that indicates the presence of the blossom must be included in the ring,
            //  since it is the only edge that is guaranteed to exist between the outermost nodes containing a and b.
            if (a == u && b == v) || (a == v && b == u) {
                ring.push(Some(e));

            // If a and b are adjacent in the cycle and one is the parent of the other in the alternating tree,
            // then the edge between them in the tree must be included in the ring,
            // since it is the only edge that is guaranteed to exist between the outermost nodes containing a and b.
            } else if self.parent[a] == Some(b) {
                ring.push(self.parent_edge[a]);
            } else if self.parent[b] == Some(a) {
                ring.push(self.parent_edge[b]);

            // Otherwise, if a and b are adjacent in the cycle but not in the tree,
            // then there may or may not be an edge between them in the base graph
            } else {
                ring.push(match self.try_tight_between(a, b)? {
                    // If there is a tight edge between the outermost nodes containing a and b, include it in the ring,
                    // since it is the only edge that is guaranteed to exist between those nodes.
                    Some(edge) => Some(edge),

                    // If there is no tight edge between the outermost nodes containing a and b,
                    // then there is no edge between them in the base graph
                    None => self.try_any_edge(a, b)?,
                });
            }
        }

        // Append the pseudonodes, inheriting the base's tree/match attributes
        self.duals.push(0);
        self.container.push(None);
        self.outer.push(true);
        self.label.push(Label::Even);
        self.parent.push(self.parent[base]);
        self.parent_edge.push(self.parent_edge[base]);
        self.mate.push(self.mate[base]);
        self.mate_edge.push(self.mate_edge[base]);

        // Record the cycle information for the new pseudonode and update the ring edges for the new pseudonode.
        self.cycle_of.insert(c, cycle.clone());
        for k in 0..n {
            let a = cycle[k];
            let b = cycle[(k + 1) % n];
            if a != b
                && let Some(re) = ring[k]
            {
                self.ring_edge.insert(edge_key(a, b), re);
            }
        }

        // Update the mate of the new pseudonode to match the base's mate
        // and update the mate of the partner of the base to point to the new pseudonode.
        if let Some(partner) = self.mate[base] {
            self.mate[partner] = Some(c);
        }

        // Update the container and outer status of the nodes in the cycle to reflect that they are now contained in the new pseudonode
        // and update the parent pointers of any vertices that are adjacent to the cycle to point to the new pseudonode instead
        // of the vertices in the cycle.
        for &m in &cycle {
            self.container[m] = Some(c);
            self.outer[m] = false;
        }

        // Update the parent pointers of any vertices that are adjacent to the cycle to point to the new pseudonode instead
        for x in 0..self.total_nodes() {
            if self.outer[x]
                && let Some(p) = self.parent[x]
                && cycle.contains(&p)
            {
                self.parent[x] = Some(c);
            }
        }

        Ok(c)
    }

    /// Expands an odd pseudonode (blossom) with zero dual value back into its constituent nodes in the derived graph.
    ///
    /// # Arguments
    /// * `v` - The index of the odd pseudonode in the derived graph to be expanded. This pseudonode must have zero dual value
    ///   and must currently be labeled as odd.
    ///
    /// # Returns
    /// * `Result<(), MatcherError>` - A result indicating success or failure of the expansion process.
    ///   Returns `Ok(())` if the expansion was successful, or an error if the pseudonode cannot be expanded due to invalid state
    ///   or if the cycle information is missing.
    fn try_expand(&mut self, v: usize) -> Result<(), MatcherError> {
        let cycle = self
            .cycle_of
            .get(&v)
            .cloned()
            .ok_or(MatcherError::NodeNotInBlossom(v))?;
        let n = cycle.len();
        let base = cycle[0];

        // If the pseudonode v has a parent edge, try to find a tight edge between the outermost node containing v and
        // the outermost node containing its parent, since that is the only edge that is guaranteed to exist between those
        // nodes and would be needed to maintain the alternating tree structure after expansion.
        let p = self.parent[v];
        let mut pe = self.parent_edge[v];
        if let Some(pp) = p
            && pe.is_none()
        {
            pe = self.try_tight_between(v, pp)?;
        }

        // If the pseudonode v has a mate, try to find a tight edge between the outermost node containing v and
        // the outermost node containing its mate, since that is the only edge that is guaranteed to exist between those
        // nodes and would be needed to maintain the matching structure after expansion.
        let ch = self.mate[v];
        let mut me = self.mate_edge[v];
        if let Some(cc) = ch
            && me.is_none()
        {
            // Orient the recovered edge so its first endpoint lies inside v.
            me = match self.try_tight_between(v, cc)? {
                Some((x, y)) => {
                    if self.try_outermost(x)? != v {
                        Some((y, x))
                    } else {
                        Some((x, y))
                    }
                }
                None => None,
            }
        }

        // Helper function to find the member of the cycle that corresponds to a given edge
        // from the pseudonode v to another vertex in the tree or matching.
        let member_for =
            |edge: Option<(usize, usize)>, this: &Self| -> Result<usize, MatcherError> {
                match edge {
                    None => Ok(base),

                    Some((a, b)) => {
                        // The member of the cycle that corresponds to the edge must be the one that is connected
                        //  to the outermost node containing v by that edge,  since that is the only edge that is guaranteed
                        //  to exist between the outermost node containing v and the nodes in the cycle.
                        let o = if this.try_outermost(a)? == v { a } else { b };
                        this.try_outermost_within(v, o)
                    }
                }
            };

        let s_par = member_for(pe, self)?;
        let s_ch = member_for(me, self)?;

        // Update the container and outer status of the nodes in the cycle to reflect that they are no longer contained in the pseudonode v.
        self.outer[v] = false;
        for &m in &cycle {
            self.container[m] = None;
            self.outer[m] = true;
        }

        if let (Some(cc), Some((a, b))) = (ch, me) {
            let (a, b) = if self.try_outermost(a)? != s_ch {
                (b, a)
            } else {
                (a, b)
            };
            self.mate[s_ch] = Some(cc);
            self.mate[cc] = Some(s_ch);
            self.mate_edge[s_ch] = Some((a, b));
            self.mate_edge[cc] = Some((b, a));
        }

        // v is attached to the tree by two edges: a non-matching edge to parent p
        // (entering at s_par) and the matching edge to child ch (entering at s_ch).
        // Splice v's odd cycle into the tree: the arc with an EVEN number of edges
        // (P) joins the tree so p - s_par - ... - s_ch - ch stays alternating; the
        // members not on P are matched internally along the cycle.
        let i_par = cycle
            .iter()
            .position(|&x| x == s_par)
            .ok_or(MatcherError::NodeNotInBlossom(s_par))?;

        let i_ch = cycle
            .iter()
            .position(|&x| x == s_ch)
            .ok_or(MatcherError::NodeNotInBlossom(s_ch))?;

        let arc = |step: isize| -> Vec<usize> {
            let mut out = Vec::new();
            let mut i = i_par as isize;
            loop {
                out.push(cycle[i as usize]);
                if i as usize == i_ch {
                    break;
                }
                i = (i + step).rem_euclid(n as isize);
            }
            out
        };

        let path_p: Vec<usize>;

        if s_par == s_ch {
            path_p = vec![s_par];

            // Both tree edges attach at the same member: the tree path is that one
            // member; all others are matched internally, paired around the cycle.
            let rest: Vec<usize> = (0..n - 1).map(|k| cycle[(i_par + 1 + k) % n]).collect();
            self.try_match_consecutive(&rest)?;
        } else {
            let fwd = arc(1);
            let bwd = arc(-1);
            let p_arc = if (fwd.len() - 1) % 2 == 0 { fwd } else { bwd };
            let on_p: HashSet<usize> = p_arc.iter().copied().collect();

            // Members not on P form one contiguous arc. Collect them in adjacency
            // order: start at the non-P member whose cycle-predecessor is on P, then
            // walk forward. match_consecutive pairs neighbours in this list, so they
            // must be cycle-adjacent for their connecting edge to be tight; raw
            // cycle-index order could bridge the wrap-around and pair non-adjacent
            // members over a non-tight edge.
            let mut rest = Vec::new();
            let mut start: Option<usize> = None;
            for k in 0..n {
                if !on_p.contains(&cycle[k]) && on_p.contains(&cycle[(k + n - 1) % n]) {
                    start = Some(k);
                    break;
                }
            }

            if let Some(s) = start {
                let mut i = s;
                while !on_p.contains(&cycle[i]) {
                    rest.push(cycle[i]);
                    i = (i + 1) % n;
                }
            }
            self.try_match_consecutive_arc(&p_arc)?;
            self.try_match_consecutive(&rest)?;
            path_p = p_arc;
        }

        // Splice P into the tree: s_par takes v's odd role under p, labels alternate
        // along P up to s_ch, which carries the matching edge to ch.
        self.parent[s_par] = p;
        self.parent_edge[s_par] = pe;
        self.label[s_par] = Label::Odd;
        let mut lab = Label::Odd;
        for idx in 1..path_p.len() {
            self.parent[path_p[idx]] = Some(path_p[idx - 1]);
            lab = if lab == Label::Odd {
                Label::Even
            } else {
                Label::Odd
            };
            self.label[path_p[idx]] = lab;
        }
        if let Some(cc) = ch {
            self.parent[cc] = Some(s_ch);
        }
        let on_path: HashSet<usize> = path_p.iter().copied().collect();

        // Members not on the tree path leave the tree (now matched internally).
        for &x in &cycle {
            if !on_path.contains(&x) {
                self.label[x] = Label::Free;
                self.parent[x] = None;
            }
        }

        Ok(())
    }

    /// Finds the direct member of pseudonode `v`'s cycle on the containment chain of `orig`.
    ///
    /// # Arguments
    /// * `v` - The pseudonode whose cycle members are searched.
    /// * `orig` - The original (or inner) node whose containing cycle member is sought.
    ///
    /// # Returns
    /// * `Result<usize, MatcherError>` - The direct member of `v`'s cycle that contains `orig`;
    ///   falls back to `v`'s base member if `orig` is not contained in `v`.
    fn try_outermost_within(&self, v: usize, orig: usize) -> Result<usize, MatcherError> {
        let mut cur = orig;
        while self.container[cur] != Some(v) {
            match self.container[cur] {
                Some(c) => cur = c,
                None => {
                    return self
                        .cycle_of
                        .get(&v)
                        .and_then(|cyc| cyc.first().copied())
                        .ok_or(MatcherError::NodeNotInBlossom(v));
                }
            }
        }

        Ok(cur)
    }

    /// Matches consecutive nodes in the given list, assuming they are connected by tight edges in the derived graph.
    ///
    /// # Arguments
    /// * `nodes` - A slice of node indices in the derived graph that are to be matched consecutively.
    ///   It is assumed that for each pair of consecutive nodes in this list, there exists a tight edge connecting them in the derived graph.
    ///
    /// # Returns
    /// * `Result<(), MatcherError>` - A result indicating success or failure of the matching process.
    fn try_match_consecutive(&mut self, nodes: &[usize]) -> Result<(), MatcherError> {
        let mut k = 0;
        while k + 1 < nodes.len() {
            let (a, b) = (nodes[k], nodes[k + 1]);
            if let Some(e) = self.try_tight_cycle_edge(a, b)? {
                self.try_set_match(a, b, e)?;
            }

            k += 2
        }

        Ok(())
    }

    /// Matches consecutive nodes in the given list, assuming they are connected by tight edges in the derived graph,
    /// and that these nodes form an arc in a cycle.
    ///
    /// # Arguments
    /// * `p_arc` - A slice of node indices in the derived graph that are to be matched consecutively, where these nodes form an arc in a cycle.
    ///
    /// # Returns
    /// * `Result<(), MatcherError>` - A result indicating success or failure of the matching process.
    fn try_match_consecutive_arc(&mut self, p_arc: &[usize]) -> Result<(), MatcherError> {
        let mut k = 0;
        while k + 1 < p_arc.len() {
            let (a, b) = (p_arc[k], p_arc[k + 1]);
            if let Some(e) = self.try_tight_cycle_edge(a, b)? {
                self.try_set_match(a, b, e)?;
            }

            k += 2
        }

        Ok(())
    }

    /// Finds a tight edge between two nodes in the derived graph, first checking the ring edges for blossoms
    /// and then falling back to any tight edge between the outermost nodes.
    ///
    /// # Arguments
    /// * `a` - The index of the first vertex in the derived graph.
    /// * `b` - The index of the second vertex in the derived graph.
    ///
    /// # Returns
    /// * `Result<Option<(usize, usize)>, MatcherError>` - A result containing an optional tuple representing the original edge (i, j)
    ///   in the base graph that connects the outermost nodes containing `a` and `b`, or an error if the search for a tight edge fails due to invalid state.
    fn try_tight_cycle_edge(
        &self,
        a: usize,
        b: usize,
    ) -> Result<Option<(usize, usize)>, MatcherError> {
        // Prefer the edge recorded when the blossom formed: it was tight then and is
        // the correct cycle edge even if later dual changes made it slack. Re-searching
        // for a currently-tight edge could pick a wrong, non-cycle edge.
        if let Some(&e) = self.ring_edge.get(&edge_key(a, b)) {
            return Ok(Some(e));
        }
        match self.try_tight_between(a, b)? {
            Some(edge) => Ok(Some(edge)),
            None => self.try_any_edge(a, b),
        }
    }

    /// Extracts the matching from the derived graph back to the original graph, populating `self.mate` with the final matches.
    ///
    /// # Returns
    /// * `Result<(), MatcherError>` - A result indicating success or failure of the extraction process.
    fn try_extract_matching(&mut self) -> Result<(), MatcherError> {
        for c in (self.n0..self.total_nodes()).rev() {
            if !self.outer[c] || !self.is_pseudonode(c) {
                continue;
            }
            self.try_expand_final(c)?;
        }
        Ok(())
    }

    /// Expands a pseudonode (blossom) in the final matching extraction phase,
    /// ensuring that the matches are correctly set for the constituent nodes.
    ///
    /// # Arguments
    /// * `c` - The index of the pseudonode in the derived graph to be expanded.
    ///   This pseudonode must currently be outer and must represent a blossom that was shrunk during the algorithm.
    ///
    /// # Returns
    /// * `Result<(), MatcherError>` - A result indicating success or failure of the expansion process.
    ///   Returns `Ok(())` if the expansion was successful and the matches were correctly set for the constituent nodes,
    ///   or an error if the pseudonode cannot be expanded due to invalid state or if the cycle information is missing.
    fn try_expand_final(&mut self, c: usize) -> Result<(), MatcherError> {
        let cycle = self
            .cycle_of
            .get(&c)
            .cloned()
            .ok_or(MatcherError::NodeNotInBlossom(c))?;
        let n = cycle.len();
        let base = cycle[0];

        let me = self.mate_edge[c];
        let s_out = match me {
            Some((o0, _)) => self.try_outermost_within(c, o0)?,

            None => base,
        };

        self.outer[c] = false;
        for &m in &cycle {
            self.container[m] = None;
            self.outer[m] = true;
        }

        if let Some((o0, o1)) = me {
            let cc = self.mate[c].ok_or(MatcherError::MissingMate(c))?;
            self.mate[s_out] = Some(cc);
            self.mate[cc] = Some(s_out);
            self.mate_edge[s_out] = Some((o0, o1));
            self.mate_edge[cc] = Some((o1, o0));
        }

        let j = cycle
            .iter()
            .position(|&x| x == s_out)
            .ok_or(MatcherError::NodeNotInBlossom(s_out))?;
        let rest: Vec<usize> = (0..n - 1).map(|k| cycle[(j + 1 + k) % n]).collect();
        self.try_match_consecutive(&rest)?;
        Ok(())
    }

    /// Returns a slice of the original mate array corresponding to the base graph nodes.
    ///
    /// # Returns
    /// * `&[Option<usize>]` - A slice of the `self.mate` array that corresponds to the original nodes in the base graph (indices 0 to n0-1).
    fn original_mate(&self) -> &[Option<usize>] {
        &self.mate[..self.n0]
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

/// Helper function to update the current minimum value with a candidate value, returning the smaller of the two.
///
/// # Arguments
/// * `current` - An `Option<i64>` representing the current minimum value, which may be `None` if no minimum has been established yet.
/// * `candidate` - An `i64` representing the candidate value to compare against the current minimum.
///
/// # Returns
/// * `i64` - The smaller of the current minimum value (if it exists) and the candidate value. If `current` is `None`,
///   the function returns the candidate value.
fn take_min(current: Option<i64>, candidate: i64) -> i64 {
    match current {
        Some(value) => value.min(candidate),
        None => candidate,
    }
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
mod tests;
