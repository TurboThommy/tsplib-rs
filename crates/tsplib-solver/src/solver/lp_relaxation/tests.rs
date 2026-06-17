use std::collections::HashSet;

#[cfg(test)]
use tsplib_core::{
    enums::{DistanceSource, EdgeWeightType, ProblemType},
    models::{Node, TsplibInstance},
};

use crate::{
    errors::SimplexError,
    solver::lp_relaxation::{
        add_subtour_cut, assert_canonical, assert_dimensions, canonicalize_cost, cut_edges_for_set,
        edge_col, try_build_tableau, try_dual_simplex, try_primal_simplex, try_solve_initial,
    },
};

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

    let tableau = try_build_tableau(&problem).expect("Tableau should be built successfully");
    let (a, b, c, basis) = tableau;

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

    let (a, _, _, basis) =
        try_build_tableau(&problem).expect("Tableau should be built successfully");
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

    let (_, x) = try_solve_initial(&problem).expect("Initial LP should be solved successfully");

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
    let ((mut a, mut b, mut c, mut basis), _) =
        try_solve_initial(&problem).expect("Initial LP should be solved successfully");

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
