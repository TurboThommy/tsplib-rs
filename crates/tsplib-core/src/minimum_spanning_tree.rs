use itertools::Itertools;

use crate::{enums::ConversionError, models::TsplibInstance};

/// A union-find (disjoint set) data structure for Kruskal's algorithm.
struct UnionFind {
    parent: Vec<usize>,
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
        let root_u = self.find(u);
        let root_v = self.find(v);

        if root_u == root_v {
            return false;
        }

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
pub fn try_get_mst_kruskal(
    tsplib_instance: &TsplibInstance,
) -> Result<Vec<(usize, usize, i32)>, ConversionError> {
    let matrix = &tsplib_instance.adjacency_matrix;

    if matrix.is_empty() {
        println!("Adjacency matrix is empty, cannot compute MST");
        return Err(ConversionError::EmptyAdjacencyMatrix);
    }

    // resulting edges of the MST
    let mut t: Vec<(usize, usize, i32)> = Vec::new();

    let n = matrix.len();

    // get edges of the triangular matrix without diagonal and sort them by distance in ascending order
    let edges = (0..(n - 1))
        .flat_map(|i| ((i + 1)..n).map(move |j| (i + 1, j + 1, matrix[i][j])))
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

    Ok(t)
}
