use std::collections::{HashMap, HashSet};

use tsplib_core::context::ExecutionContext;
#[cfg(test)]
use tsplib_core::{
    enums::{DistanceSource, EdgeWeightType, ProblemType},
    models::{Node, TsplibInstance},
};

use crate::{
    HeldKarp, TspSolver,
    errors::{SimplexError, SolverError},
    solver::linearprogramming::{
        add_subtour_cut, assert_canonical, assert_dimensions, branch_and_bound,
        build_weight_matrix, canonicalize_cost, cut_edges_for_set, edge_col, edge_key,
        find_fractional_edge, min_cut, reconstruct_tour, try_build_tableau, try_dual_simplex,
        try_primal_simplex, try_solve_initial,
    },
};

const EPS: f64 = 1e-9;

fn make_test_instance() -> TsplibInstance {
    TsplibInstance {
        problem_id: "test".to_string(),
        name: "test".to_string(),
        problem_type: ProblemType::TSP,
        nodes: vec![
            Node {
                id: 1,
                x: 0.0,
                y: 0.0,
                z: None,
            },
            Node {
                id: 2,
                x: 1.0,
                y: 1.0,
                z: None,
            },
            Node {
                id: 3,
                x: 2.0,
                y: 2.0,
                z: None,
            },
            Node {
                id: 4,
                x: 3.0,
                y: 3.0,
                z: None,
            },
        ],
        distance_source: DistanceSource::Geometric(EdgeWeightType::Euc2D),
        fixed_edges: None,
    }
}

fn make_clustered_instance() -> TsplibInstance {
    TsplibInstance {
        problem_id: "clustered".to_string(),
        name: "clustered".to_string(),
        problem_type: ProblemType::TSP,
        nodes: vec![
            Node {
                id: 1,
                x: 0.0,
                y: 0.0,
                z: None,
            },
            Node {
                id: 2,
                x: 1.0,
                y: 0.0,
                z: None,
            },
            Node {
                id: 3,
                x: 0.5,
                y: 1.0,
                z: None,
            },
            Node {
                id: 4,
                x: 1000.0,
                y: 0.0,
                z: None,
            },
            Node {
                id: 5,
                x: 1001.0,
                y: 0.0,
                z: None,
            },
            Node {
                id: 6,
                x: 1000.5,
                y: 1.0,
                z: None,
            },
        ],
        distance_source: DistanceSource::Geometric(EdgeWeightType::Euc2D),
        fixed_edges: None,
    }
}

fn solve_lp_relaxation_counted(problem: &TsplibInstance) -> Result<(Vec<f64>, usize), SolverError> {
    let node_count = problem.nodes.len();
    let fixed_edges = HashMap::new();
    let ((mut a, mut b, mut c, mut basis, _), mut x) = try_solve_initial(problem, &fixed_edges)
        .unwrap()
        .expect("Initial LP should be solved successfully");

    let mut rounds = 0;
    loop {
        let w = build_weight_matrix(&x, node_count);
        let (cut_value, s_zero) = min_cut(&w, node_count);
        if cut_value >= 2.0 - EPS {
            break;
        }
        let s: HashSet<usize> = s_zero.iter().map(|&i| i + 1).collect();
        let cut = cut_edges_for_set(node_count, &s);
        add_subtour_cut(&mut a, &mut b, &mut c, &mut basis, &cut);
        x = try_dual_simplex(&mut a, &mut b, &mut c, &mut basis)?;
        rounds += 1;
    }
    Ok((x, rounds))
}

#[allow(clippy::needless_range_loop)]
fn assert_subtour_free_and_feasible(x: &[f64], n: usize) {
    for mask in 1u32..((1 << n) - 1) {
        let s: HashSet<usize> = (1..=n).filter(|&i| mask & (1 << (i - 1)) != 0).collect();
        let crossing: f64 = cut_edges_for_set(n, &s).iter().map(|&k| x[k]).sum();
        assert!(crossing >= 2.0 - 1e-9, "Cut {s:?} verletzt: {crossing}");
    }
    for v in 1..=n {
        let deg: f64 = (1..=n).filter(|&w| w != v).map(|w| x[edge_col(v, w)]).sum();
        assert!((deg - 2.0).abs() < 1e-9, "Knoten {v}: Grad {deg}");
    }
    for k in 0..(n * (n - 1) / 2) {
        assert!(
            x[k] >= -1e-9 && x[k] <= 1.0 + 1e-9,
            "Kante {k} außerhalb [0,1]: {}",
            x[k]
        );
    }
}

#[test]
fn test_primal_simplex() {
    let mut a = vec![
        vec![1.0, 1.0, 1.0, 0.0, 0.0],
        vec![6.0, 9.0, 0.0, 1.0, 0.0],
        vec![0.0, 1.0, 0.0, 0.0, 1.0],
    ];

    let mut b = vec![100.0, 720.0, 60.0];
    let mut c = vec![-10.0, -20.0, 0.0, 0.0, 0.0];
    let mut basis = vec![2, 3, 4];

    let c_orig = c.clone();

    let x = try_primal_simplex(&mut a, &mut b, &mut c, &mut basis)
        .expect("Primal simplex should find an optimal solution");

    assert!((x[0] - 30.0).abs() < 1e-9); // x1
    assert!((x[1] - 60.0).abs() < 1e-9); // x2
    assert!((x[2] - 10.0).abs() < 1e-9); // slack variable for first constraint
    assert!((x[3].abs() < 1e-9)); // slack variable for second constraint
    assert!((x[4].abs() < 1e-9)); // slack variable for third constraint

    assert_eq!(basis, vec![2, 0, 1]);

    let obj: f64 = c_orig.iter().zip(&x).map(|(&c_i, x_i)| c_i * x_i).sum();
    assert!((obj + 1500.0).abs() < 1e-9); // min -1500 = max 1500
}

#[test]
fn test_primal_simplex_unbounded() {
    let mut a = vec![vec![-1.0, 1.0]];
    let mut b = vec![1.0];
    let mut c = vec![-1.0, 0.0];
    let mut basis = vec![1];

    assert_eq!(
        try_primal_simplex(&mut a, &mut b, &mut c, &mut basis),
        Err(SimplexError::Unbounded)
    );
}

#[test]
fn test_dual_simplex() {
    let mut a = vec![
        vec![-1.0, -1.0, 1.0, 0.0, 0.0],
        vec![-3.0, -1.0, 0.0, 1.0, 0.0],
        vec![1.0, 1.0, 0.0, 0.0, 1.0],
    ];

    let mut b = vec![-8.0, -12.0, 10.0];
    let mut c = vec![-2.0, -1.0, 0.0, 0.0, 0.0];
    let mut basis = vec![2, 3, 4];

    let c_orig = c.clone();

    let x = try_dual_simplex(&mut a, &mut b, &mut c, &mut basis)
        .expect("Dual simplex should find an optimal solution");

    assert!((x[0] - 10.0).abs() < 1e-9); // x1
    assert!(x[1].abs() < 1e-9); // x2
    assert!((x[2] - 2.0).abs() < 1e-9); // slack variable for first constraint
    assert!((x[3] - 18.0).abs() < 1e-9); // slack variable for second constraint
    assert!(x[4].abs() < 1e-9); // slack variable for third constraint

    assert_eq!(basis, vec![2, 3, 0]);

    let obj: f64 = c_orig.iter().zip(&x).map(|(&c_i, x_i)| c_i * x_i).sum();
    assert!((obj + 20.0).abs() < 1e-9);
}

#[test]
fn test_dual_simplex_infeasible() {
    let mut a = vec![vec![1.0, 1.0, 1.0, 0.0], vec![1.0, 0.0, 0.0, 1.0]];
    let mut b = vec![-1.0, 5.0];
    let mut c = vec![1.0, 1.0, 0.0, 0.0];
    let mut basis = vec![2, 3];

    assert_eq!(
        try_dual_simplex(&mut a, &mut b, &mut c, &mut basis),
        Err(SimplexError::Infeasible)
    )
}

#[test]
fn test_build_tableau() {
    let problem = make_test_instance();
    let fixed_edges = HashMap::new();

    let (a, b, c, basis, _) =
        try_build_tableau(&problem, &fixed_edges).expect("Tableau should be built successfully");

    // Check dimensions of the tableau
    // 4 degree constraints + 6 upper-bound constraints
    assert_eq!(a.len(), 10);
    // 16 variables (6 edges + 4 artificial variables + 6 slack variables)
    assert_eq!(a[0].len(), 16);
    assert_eq!(b.len(), 10);
    assert_eq!(c.len(), 16);
    assert_eq!(basis.len(), 10);

    // Check degree constraints
    let mut deg_1 = vec![0.0; 16];
    deg_1[0] = 1.0;
    deg_1[1] = 1.0;
    deg_1[3] = 1.0;
    deg_1[12] = 1.0;

    let mut deg_2 = vec![0.0; 16];
    deg_2[0] = 1.0;
    deg_2[2] = 1.0;
    deg_2[4] = 1.0;
    deg_2[13] = 1.0;

    let mut deg_3 = vec![0.0; 16];
    deg_3[1] = 1.0;
    deg_3[2] = 1.0;
    deg_3[5] = 1.0;
    deg_3[14] = 1.0;

    let mut deg_4 = vec![0.0; 16];
    deg_4[3] = 1.0;
    deg_4[4] = 1.0;
    deg_4[5] = 1.0;
    deg_4[15] = 1.0;

    assert_eq!(a[0], deg_1);
    assert_eq!(a[1], deg_2);
    assert_eq!(a[2], deg_3);
    assert_eq!(a[3], deg_4);

    // Check upper-bound constraints
    let mut edge_12 = vec![0.0; 16];
    edge_12[0] = 1.0;
    edge_12[6] = 1.0;

    let mut edge_13 = vec![0.0; 16];
    edge_13[1] = 1.0;
    edge_13[7] = 1.0;

    let mut edge_23 = vec![0.0; 16];
    edge_23[2] = 1.0;
    edge_23[8] = 1.0;

    let mut edge_14 = vec![0.0; 16];
    edge_14[3] = 1.0;
    edge_14[9] = 1.0;

    let mut edge_24 = vec![0.0; 16];
    edge_24[4] = 1.0;
    edge_24[10] = 1.0;

    let mut edge_34 = vec![0.0; 16];
    edge_34[5] = 1.0;
    edge_34[11] = 1.0;

    assert_eq!(a[4], edge_12);
    assert_eq!(a[5], edge_13);
    assert_eq!(a[6], edge_23);
    assert_eq!(a[7], edge_14);
    assert_eq!(a[8], edge_24);
    assert_eq!(a[9], edge_34);

    assert_dimensions(&a, &b, &c);
    assert_canonical(&a, &b, &c, &basis);
}

#[test]
#[allow(clippy::needless_range_loop)]
fn test_canonicalize_phase_one() {
    let problem = make_test_instance();
    let fixed_edges = HashMap::new();

    let (a, _, _, basis, _) =
        try_build_tableau(&problem, &fixed_edges).expect("Tableau should be built successfully");
    let n = a[0].len();
    let e = 6;

    let mut c = vec![0.0; n];
    for v in 0..4 {
        c[2 * e + v] = 1.0;
    }

    canonicalize_cost(&a, &mut c, &basis);

    for k in 0..e {
        assert!((c[k] + 2.0).abs() < 1e-9, "Edge {k}: {}", c[k]);
    }

    for k in e..2 * e {
        assert!(c[k].abs() < 1e-9, "Slack {k}");
    }

    for k in 2 * e..n {
        assert!(c[k].abs() < 1e-9, "Artificial {k}");
    }
}

#[test]
#[allow(clippy::needless_range_loop)]
fn test_solve_initial_lp() {
    let problem = make_test_instance();
    let node_count = problem.nodes.len();
    let e = node_count * (node_count - 1) / 2;
    let artificial_offset = 2 * e;
    let fixed_edges = HashMap::new();
    let (_, x) = try_solve_initial(&problem, &fixed_edges)
        .unwrap()
        .expect("Initial LP should be solved successfully");

    let artificial_sum = (0..node_count)
        .map(|v| x[artificial_offset + v])
        .sum::<f64>();
    assert!(
        artificial_sum.abs() < 1e-9,
        "Artificial variables not eliminated: {artificial_sum}"
    );

    for k in 0..e {
        assert!(
            x[k] >= -1e-9 && x[k] <= 1.0 + 1e-9,
            "Edge {k} out of bounds [0, 1]: {}",
            x[k]
        );
    }

    for v in 1..=node_count {
        let degree = (1..=node_count)
            .filter(|&w| w != v)
            .map(|w| x[edge_col(v, w)])
            .sum::<f64>();

        assert!(
            (degree - 2.0).abs() < 1e-9,
            "Node {v} has degree {degree}, expected 2"
        );
    }
}

#[test]
fn test_add_subtour_cut() {
    let problem = make_test_instance();
    let fixed_edges = HashMap::new();
    let ((mut a, mut b, mut c, mut basis, _), _) = try_solve_initial(&problem, &fixed_edges)
        .unwrap()
        .expect("Initial LP should be solved successfully");

    let s: HashSet<usize> = [1, 2].into_iter().collect();
    let cut = cut_edges_for_set(4, &s);
    add_subtour_cut(&mut a, &mut b, &mut c, &mut basis, &cut);

    assert_dimensions(&a, &b, &c);
    assert_canonical(&a, &b, &c, &basis);

    let x = try_dual_simplex(&mut a, &mut b, &mut c, &mut basis)
        .expect("Dual simplex should solve after adding subtour cut");

    for v in 1..=4 {
        let deg = (1..=4)
            .filter(|&w| w != v)
            .map(|w| x[edge_col(v, w)])
            .sum::<f64>();
        assert!((deg - 2.0).abs() < 1e-9);
    }
    let crossing = cut.iter().map(|&k| x[k]).sum::<f64>();
    assert!(
        crossing >= 2.0 - 1e-9,
        "Subtour cut not satisfied, crossing value: {crossing}"
    );
}

#[test]
fn test_min_cut() {
    let mut w = vec![vec![0.0; 4]; 4];
    let edges = [
        (0, 1, 2.0),
        (0, 2, 3.0),
        (1, 2, 1.0),
        (1, 3, 3.0),
        (2, 3, 2.0),
    ];
    for (i, j, x) in edges {
        w[i][j] = x;
        w[j][i] = x;
    }

    let (value, s) = min_cut(&w, 4);
    assert!(
        (value - 5.0).abs() < 1e-9,
        "Min cut value should be 5.0, got {value}"
    );
    assert!(
        !s.is_empty() && s.len() < 4,
        "Min cut has to be a proper subset"
    );
}

#[test]
fn test_separation_no_subtours() {
    let problem = make_test_instance();
    let n = problem.nodes.len();

    let (x, _rounds) = solve_lp_relaxation_counted(&problem).unwrap();

    assert_subtour_free_and_feasible(&x, n);
}

#[test]
fn test_separation_runs_at_least_once() {
    let problem = make_clustered_instance();
    let n = problem.nodes.len();

    let (x, rounds) = solve_lp_relaxation_counted(&problem).unwrap();

    assert!(
        rounds >= 1,
        "kein Subzyklus erkannt — Schleife lief {rounds} Runden"
    );
    assert_subtour_free_and_feasible(&x, n);
}

#[test]
fn test_find_fractional_edge_found() {
    let edges = vec![(2, 1, 1.0), (3, 1, 0.5), (4, 2, 1.0)];
    assert_eq!(find_fractional_edge(&edges), Some((3, 1)));
}

#[test]
fn test_find_fractional_edge_all_integral() {
    let edges = vec![(2, 1, 1.0), (3, 1, 1.0), (4, 2, 1.0)];
    assert_eq!(find_fractional_edge(&edges), None);
}

#[test]
fn test_find_fractional_edge_boundary_values() {
    // exakt 0 und exakt 1 sind NICHT fraktional
    let edges = vec![(2, 1, 1.0), (3, 1, 0.0)];
    assert_eq!(find_fractional_edge(&edges), None);
}

#[test]
fn test_reconstruct_tour_simple_cycle() {
    let problem = make_test_instance(); // n=4
    // Kreis 1-2-3-4-1, Kanten ungeordnet und in beliebiger Knotenrichtung
    let edges = vec![(3, 2), (4, 1), (2, 1), (4, 3)];

    let solution = reconstruct_tour(&problem, &edges).unwrap();

    // Länge stimmt, jeder Knoten genau einmal
    assert_eq!(solution.tour.len(), 4);
    let mut sorted = solution.tour.clone();
    sorted.sort();
    assert_eq!(sorted, vec![1, 2, 3, 4]);
}

#[test]
fn test_reconstruct_tour_edges_are_connected() {
    let problem = make_test_instance();
    let edges = vec![(3, 2), (4, 1), (2, 1), (4, 3)];
    let edge_set: HashSet<(usize, usize)> = edges.iter().map(|&(i, j)| edge_key(i, j)).collect();

    let solution = reconstruct_tour(&problem, &edges).unwrap();
    let n = solution.tour.len();

    // jede aufeinanderfolgende Paarung (inkl. Rückkante) muss eine echte Kante sein
    for w in 0..n {
        let from = solution.tour[w];
        let to = solution.tour[(w + 1) % n];
        assert!(
            edge_set.contains(&edge_key(from, to)),
            "Tour nutzt Nicht-Kante {from}-{to}"
        );
    }
}

#[test]
fn test_reconstruct_tour_rejects_invalid() {
    let problem = make_test_instance();
    // Knoten 1 hätte nur einen Nachbarn -> kein Hamiltonkreis
    let edges = vec![(2, 1), (3, 2), (4, 3)]; // offener Pfad, kein Kreis
    assert!(reconstruct_tour(&problem, &edges).is_err());
}

#[test]
fn test_branch_and_bound_matches_held_karp() {
    let problem = make_clustered_instance();
    let fixed = HashMap::new();
    let bb = branch_and_bound(&problem, &fixed, ExecutionContext::default())
        .unwrap()
        .unwrap();
    let hk = HeldKarp::try_new(25)
        .unwrap()
        .try_solve(&problem, 1)
        .unwrap();
    assert_eq!(bb.cost, hk.cost, "B&B-Optimum weicht von Held-Karp ab");
}
