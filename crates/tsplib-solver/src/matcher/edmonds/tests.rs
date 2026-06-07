//! Oracle-based property tests for the weighted perfect-matching algorithm.

#![cfg(test)]

use std::collections::HashSet;

use tsplib_core::{
    enums::ProblemType,
    models::{Edge, Node, TsplibInstance},
};

use crate::{
    PerfectMatchingAlgorithm, RecursiveMatching, matcher::edmonds::WeightedEdmondsMatching,
};

/// Minimal deterministic PRNG (xorshift64*). Avoids a `rand` dev-dependency and
/// keeps every failure reproducible from its seed.
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Rng(seed | 1) // avoid zero state
    }

    /// Generates the next random `u64` value using the xorshift64* algorithm.
    fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545F4914F6CDD1D)
    }

    /// Generates a random `u64` value in the range `[0, bound)`.
    ///
    /// # Arguments
    /// * `bound` - The upper bound (exclusive) for the generated random number.
    ///
    /// # Returns
    /// * `u64` - A random number in the range `[0, bound)`.
    fn below(&mut self, bound: u64) -> u64 {
        self.next_u64() % bound
    }
}

struct RandomInstance {
    coords: Vec<(i64, i64)>,
    instance: TsplibInstance,
    /// Node ids (1-based), playing the role of "odd vertices"
    vertices: Vec<usize>,
}

/// Calculates the Euclidean distance between two points in 2D space, rounding to the nearest integer.
///
/// # Arguments
/// * `a` - The coordinates of the first point as a tuple `(i64, i64)`.
/// * `b` - The coordinates of the second point as a tuple `(i64, i64)`.
///
/// # Returns
/// * `i32` - The rounded Euclidean distance between the two points.
fn euc_2d(a: (i64, i64), b: (i64, i64)) -> i32 {
    {
        let dx = (a.0 - b.0) as f64;
        let dy = (a.1 - b.1) as f64;
        (dx * dx + dy * dy).sqrt().round() as i32
    }
}

fn random_instance(rng: &mut Rng, n: usize, coord_max: i64) -> RandomInstance {
    assert!(
        n.is_multiple_of(2),
        "matching needs an even number of vertices"
    );

    let coords: Vec<(i64, i64)> = (0..n)
        .map(|_| {
            (
                rng.below(coord_max as u64) as i64,
                rng.below(coord_max as u64) as i64,
            )
        })
        .collect();

    let mut adjacency_matrix = vec![vec![0i32; n]; n];
    for i in 0..n {
        for j in (i + 1)..n {
            let d = euc_2d(coords[i], coords[j]);
            adjacency_matrix[i][j] = d;
            adjacency_matrix[j][i] = d;
        }
    }

    let nodes: Vec<Node> = (0..n)
        .map(|i| Node {
            id: i + 1,
            x: coords[i].0 as f64,
            y: coords[i].1 as f64,
            z: None,
        })
        .collect();

    let instance = TsplibInstance {
        problem_id: "oracle".to_string(),
        name: "oracle".to_string(),
        problem_type: ProblemType::TSP,
        nodes,
        adjacency_matrix,
        fixed_edges: None,
    };

    RandomInstance {
        coords,
        instance,
        vertices: (1..=n).collect(),
    }
}

/// Sums edge weights by looking each one up in the instance, so the comparison
/// does not trust the weight the matcher stored on the edge.
///
/// # Arguments
/// * `edges` - A slice of `Edge` structs representing the edges in the matching.
/// * `instance` - The `TsplibInstance` containing the distance information for the nodes.
///
/// # Returns
/// * `i64` - The total cost of the matching, calculated by summing the distances of the edges as defined in the instance.
fn matching_cost(edges: &[Edge], instance: &TsplibInstance) -> i64 {
    edges
        .iter()
        .map(|e| {
            i64::from(
                instance
                    .try_get_distance(e.u, e.v)
                    .expect("matched edge must have a valid distance"),
            )
        })
        .sum()
}

/// Asserts the edge set is a genuine perfect matching of vertices.
///
/// # Arguments
/// * `edges` - A slice of `Edge` structs representing the edges in the matching.
/// * `vertices` - A slice of vertex indices that should be covered by the matching.
/// * `label` - A label for the assertion, used in error messages to identify which matcher is being tested.
/// * `ctx` - Additional context for the assertion, included in error messages to help diagnose failures.
fn assert_perfect(edges: &[Edge], vertices: &[usize], label: &str, ctx: &str) {
    assert_eq!(
        edges.len(),
        vertices.len() / 2,
        "{label} returned {} edges, expected {} ({ctx})",
        edges.len(),
        vertices.len() / 2
    );

    let mut seen = HashSet::new();
    for e in edges {
        assert!(
            vertices.contains(&e.u) && vertices.contains(&e.v),
            "{label} edge ({}, {}) uses a vertec outside the instance ({ctx})",
            e.u,
            e.v
        );
        assert!(seen.insert(e.u), "{label} covers {} twice ({ctx})", e.u);
        assert!(seen.insert(e.v), "{label} covers {} twice ({ctx})", e.v);
    }
    assert_eq!(
        seen.len(),
        vertices.len(),
        "{label} does not cover all vertices ({ctx})"
    );
}

/// Describes a random instance in a human-readable format, including its seed, number of vertices, and their coordinates.
///
/// # Arguments
/// * `inst` - The `RandomInstance` to describe, containing the coordinates and instance information.
/// * `seed` - The seed used to generate the random instance, included in the description for reproducibility.
///
/// # Returns
/// * `String` - A formatted string describing the random instance, including its seed, number of vertices, and their coordinates.
fn describe(inst: &RandomInstance, seed: u64) -> String {
    let coords = inst
        .coords
        .iter()
        .enumerate()
        .map(|(i, (x, y))| format!("    {}: ({x}, {y})", i + 1))
        .collect::<Vec<_>>()
        .join("\n");
    format!("seed={seed}, n={},\ncoords:\n{coords}", inst.coords.len())
}

/// Runs Blossom V if the feature is enabled; otherwise returns None.
///
/// # Arguments
/// * `inst` - The `RandomInstance` to solve with Blossom V, containing the coordinates and instance information.
///
/// # Returns
/// * `Option<Result<i64, String>>` - An optional result containing the cost of the matching found by Blossom V,
///   or an error message if Blossom V is not enabled or if it encounters an error during execution.
#[cfg(feature = "blossom-v")]
fn blossom_v_cost(inst: &RandomInstance) -> Option<Result<i64, String>> {
    use tsplib_solver::BlossomVMatching;

    use crate::BlossomVMatching;
    let ctx = describe(inst, 0);
    let res = BlossomVMatching::new()
        .try_compute(&inst.vertices, &inst.instance)
        .map_err(|e| format!("Blossom V errored: {e}"))
        .and_then(|edges| {
            assert_perfect(&edges, &inst.vertices, "Blossom V", &ctx);
            Ok(matching_cost(&edges, &inst.instance))
        });
    Some(res)
}

#[cfg(not(feature = "blossom-v"))]
fn blossom_v_cost(_inst: &RandomInstance) -> Option<Result<i64, String>> {
    None
}

fn compare_once(seed: u64, n: usize, coord_max: i64) -> Result<(), String> {
    let mut rng = Rng::new(seed);
    let inst = random_instance(&mut rng, n, coord_max);
    let ctx = describe(&inst, seed);

    let edmonds = WeightedEdmondsMatching::new()
        .try_compute(&inst.vertices, &inst.instance)
        .map_err(|e| format!("WeightedEdmonds errored: {e}\n{ctx}"))?;
    assert_perfect(&edmonds, &inst.vertices, "WeightedEdmonds", &ctx);
    let edmonds_cost = matching_cost(&edmonds, &inst.instance);

    // Primary oracle: exact brute force
    let exact = RecursiveMatching::new()
        .try_compute(&inst.vertices, &inst.instance)
        .map_err(|e| format!("RecursiveMatching errored: {e}\n{ctx}"))?;
    let exact_cost = matching_cost(&exact, &inst.instance);

    if edmonds_cost != exact_cost {
        return Err(format!(
            "cost mismatch vs exact: WeightedEdmonds={edmonds_cost}, exact={exact_cost}\n\
            Edmonds matching: {edmonds:?}\n{ctx}",
        ));
    }

    // Secondary cross-check against Blossom V if available
    if let Some(bv) = blossom_v_cost(&inst) {
        let bv_cost = bv.map_err(|e| format!("{e}\n{ctx}"))?;
        if bv_cost != exact_cost {
            return Err(format!(
                "Blossom V disagrees with exact: BlossomV={bv_cost}, exact={exact_cost}\n{ctx}"
            ));
        }
    }

    Ok(())
}

fn sweep(label: &str, n: usize, coord_max: i64, seeds: u64) {
    for seed in 1..=seeds {
        if let Err(msg) = compare_once(seed, n, coord_max) {
            panic!("[{label}] mismatch on seed {seed}:\n{msg}");
        }
    }
}

#[test]
fn oracle_n2() {
    sweep("n2", 2, 100, 200);
}

#[test]
fn oracle_n4() {
    // K4: the canonical "duals must be half-integral" case (Cook p. 155).
    sweep("n4", 4, 100, 3000);
}

#[test]
fn oracle_n6() {
    // First size where blossoms appear.
    sweep("n6", 6, 100, 3000);
}

#[test]
fn oracle_n8() {
    sweep("n8", 8, 100, 1500);
}

#[test]
fn oracle_n10() {
    sweep("n10", 10, 120, 800);
}

#[test]
fn oracle_n12() {
    sweep("n12", 12, 150, 400);
}

#[test]
fn oracle_n14() {
    sweep("n14", 14, 180, 200);
}

#[test]
fn oracle_n16() {
    // RecursiveMatching caps at 18 vertices; stay safely below.
    sweep("n16", 16, 200, 80);
}

/// Tight coordinate range forces many equal distances, where degenerate dual
/// updates (delta = 0) and nested blossoms tend to surface.
#[test]
fn oracle_n8_tight() {
    sweep("n8_tight", 8, 5, 3000);
}

#[test]
fn oracle_n12_tight() {
    sweep("n12_tight", 12, 6, 1000);
}

#[test]
#[allow(clippy::approx_constant)]
fn oracle_burma14_odd_set() {
    // Coordinates of MST-odd vertices {2,3,4,5,8,10} of burma14.
    let coords = [
        (16.47, 94.44), // 2
        (20.09, 92.54), // 3
        (22.39, 93.37), // 4
        (25.23, 97.24), // 5
        (17.20, 96.29), // 8
        (14.05, 98.12), // 10
    ];
    let n = coords.len();

    // GEO distance, matching tsplib-core::distances.
    fn latlon(x: f64, y: f64) -> (f64, f64) {
        let pi = 3.141592;
        let dx = x.trunc();
        let lat = pi * (dx + 5.0 * (x - dx) / 3.0) / 180.0;
        let dy = y.trunc();
        let lon = pi * (dy + 5.0 * (y - dy) / 3.0) / 180.0;
        (lat, lon)
    }
    fn geo(a: (f64, f64), b: (f64, f64)) -> i32 {
        let (lat1, lon1) = latlon(a.0, a.1);
        let (lat2, lon2) = latlon(b.0, b.1);
        let rrr = 6378.388;
        let q1 = (lon1 - lon2).cos();
        let q2 = (lat1 - lat2).cos();
        let q3 = (lat1 + lat2).cos();
        let arg = (0.5 * ((1.0 + q1) * q2 - (1.0 - q1) * q3)).clamp(-1.0, 1.0);
        (rrr * arg.acos() + 1.0) as i32
    }

    let mut adjacency_matrix = vec![vec![0i32; n]; n];
    for i in 0..n {
        for j in (i + 1)..n {
            let d = geo(coords[i], coords[j]);
            adjacency_matrix[i][j] = d;
            adjacency_matrix[j][i] = d;
        }
    }
    let nodes: Vec<Node> = (0..n)
        .map(|i| Node {
            id: i + 1,
            x: coords[i].0,
            y: coords[i].1,
            z: None,
        })
        .collect();
    let instance = TsplibInstance {
        problem_id: "burma14_odd".to_string(),
        name: "burma14_odd".to_string(),
        problem_type: ProblemType::TSP,
        nodes,
        adjacency_matrix,
        fixed_edges: None,
    };
    let vertices: Vec<usize> = (1..=n).collect();

    let edmonds = WeightedEdmondsMatching::new()
        .try_compute(&vertices, &instance)
        .expect("WeightedEdmonds must produce a perfect matching on burma14 odd set");
    assert_perfect(&edmonds, &vertices, "WeightedEdmonds", "burma14_odd_set");

    let exact = RecursiveMatching::new()
        .try_compute(&vertices, &instance)
        .expect("RecursiveMatching must solve burma14 odd set");

    assert_eq!(
        matching_cost(&edmonds, &instance),
        matching_cost(&exact, &instance),
        "WeightedEdmonds cost must equal the exact optimum on burma14 odd set",
    );
}
