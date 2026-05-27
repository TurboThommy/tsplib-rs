//! This module contains the main function and test functions for parsing TSP files and converting them to graph representations.
use std::time::Instant;

use itertools::Itertools;
use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};
use tsplib_core::{
    context::ExecutionContext,
    models::TsplibInstance,
    reader::{read_tsp_file, read_tsp_files},
};
use tsplib_parser::{parse, try_parse};
use tsplib_solver::TspSolver;

/// The main function serves as the entry point of the program, calling the test functions for parsing TSP files.
fn main() {
    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| {
            "tsplib_dev_runner=info,tsplib_solver=debug,tsplib_parser=debug,tsplib_core=debug"
                .into()
        }))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let tests: Vec<(&str, fn())> = vec![
        // ("test_parse", test_parse),
        // ("test_try_parse", test_try_parse),
        // (
        //     "test_edge_weight_matrix_conversion",
        //     test_edge_weight_matrix_conversion,
        // ),
        // ("test_graph_conversion", test_graph_conversion),
        // ("test_instance_to_string", test_instance_to_string),
        // ("test_greedy_solver", test_greedy_solver),
        // ("test_held_karp_solver", test_held_karp_solver),
        ("test_christofides_solver", test_christofides_solver),
        // ("test_kruskal", test_kruskal),
        // ("test_prim", test_prim),
        // ("test_boruvka", test_boruvka),
    ];

    for (name, test) in tests {
        tracing::info!(test = name, "Starting test");
        let now = Instant::now();
        test();
        tracing::info!(elapsed=?&now.elapsed(), test = name, "Test finished");
    }
}

/// Tests the `parse` function by reading TSP files from the "./data" directory, parsing them, and printing the results.
#[allow(dead_code)]
fn test_parse() {
    let instances = read_tsp_files("./data")
        .into_iter()
        .map(|(id, data)| parse(id, data))
        .collect::<Vec<_>>();

    tracing::info!(instances = instances.len(), "Parsed instances");
}

/// Tests the `try_parse` function by reading TSP files from the "./data" directory, attempting to parse them, and printing the results.
#[allow(dead_code)]
fn test_try_parse() {
    let try_instances = read_tsp_files("./data")
        .into_iter()
        .map(|(id, data)| try_parse(id, data))
        .collect::<Vec<_>>();

    tracing::info!(
        instances = try_instances.len(),
        successful_instances = try_instances.iter().filter(|r| r.is_ok()).count(),
        "Try parsed instances"
    );
}

/// Tests the conversion of TSP instances to graph representations and prints the sizes of the resulting graphs.
#[allow(dead_code)]
fn test_graph_conversion() {
    // only use instances with dimension <= 10000
    let tsp_instances = read_tsp_files("./data")
        .into_iter()
        .flat_map(|(id, data)| try_parse(id, data))
        .sorted_by_key(|i| i.dimension)
        .rev()
        .collect::<Vec<_>>();

    tracing::info!("Total instances: {}", tsp_instances.len());

    let _ = tsp_instances
        .iter()
        .flat_map(|i| {
            tracing::info!(
                instance_name = i.name,
                dimension = i.dimension,
                "Processing instance"
            );

            let start = std::time::Instant::now();
            let result = i.try_into().ok();
            let try_into_duration = start.elapsed();

            tracing::info!(
                instance_name = i.name,
                dimension = i.dimension,
                elapsed = ?try_into_duration,
                "Converted instance to graph representation"
            );

            result.map(|instance: TsplibInstance| {
                let start = std::time::Instant::now();
                let size = instance.heap_size();
                let heap_size_duration = start.elapsed();

                tracing::info!(
                    instance_name = i.name,
                    dimension = i.dimension,
                    size_bytes = size as f64 / (1024.0 * 1024.0),
                    elapsed = ?heap_size_duration,
                    "Calculated heap size of graph representation"
                );

                (i.name.clone(), size)
            })
        })
        .collect::<Vec<_>>();
}

/// Tests the conversion of TSP instances with EDGE_WEIGHT_SECTION to adjacency matrices and prints the results.
#[allow(dead_code)]
fn test_edge_weight_matrix_conversion() {
    let tsp_instances = read_tsp_files("./data/test_data")
        .into_iter()
        .flat_map(|(id, data)| try_parse(id, data))
        .collect::<Vec<_>>();

    let adjacency_matrices = tsp_instances
        .iter()
        .flat_map(|i| i.try_calculate_adjacency_matrix_edge_weights(ExecutionContext::default()));

    adjacency_matrices.for_each(|m| {
        tracing::info!(matrix = ?m, "Calculated adjacency matrix");
    });
}

/// Tests the to_string implementation of TSPInstance by reading a TSP file, parsing it, and printing the resulting string representation.
#[allow(dead_code)]
fn test_instance_to_string() {
    let (problem_id, data) = read_tsp_file("./data/linhp318.tsp");
    let tsp_instance = try_parse(problem_id, data).expect("failed to read instance");

    println!("{}", tsp_instance);
}

/// Tests the greedy TSP solver by creating a sample problem instance and attempting to solve it, printing the results.
#[allow(dead_code)]
fn test_greedy_solver() {
    let (problem_id, data) = read_tsp_file("./data/burma14.tsp");

    let tsp_instance = try_parse(problem_id, data).expect("failed to read instance");
    let problem_instance: TsplibInstance =
        tsp_instance.try_into().expect("failed to convert instance");

    let solver = tsplib_solver::Greedy {};
    let solution = solver
        .try_solve(&problem_instance, 1)
        .expect("failed to solve instance");

    tracing::info!(cost = solution.cost, tour = ?solution.tour, "Greedy solver completed");
}

#[allow(dead_code)]
fn test_held_karp_solver() {
    let (problem_id, data) = read_tsp_file("./data/burma14.tsp");

    let tsp_instance = try_parse(problem_id, data).expect("failed to read instance");
    let problem_instance: TsplibInstance =
        tsp_instance.try_into().expect("failed to convert instance");

    let solver = tsplib_solver::HeldKarp::try_new(25).expect("failed to create HeldKarp solver");
    let solution = solver
        .try_solve(&problem_instance, 1)
        .expect("failed to solve instance");

    tracing::info!(cost = solution.cost, tour = ?solution.tour, "Held-Karp solver completed");
}

#[allow(dead_code)]
fn test_christofides_solver() {
    let (problem_id, data) = read_tsp_file("./data/burma14.tsp");

    let tsp_instance = try_parse(problem_id, data).expect("failed to read instance");
    let problem_instance: TsplibInstance =
        tsp_instance.try_into().expect("failed to convert instance");

    let solver = tsplib_solver::Christofides::new();
    let solution = solver
        .try_solve(&problem_instance, 1)
        .expect("failed to solve instance");

    tracing::info!(cost = solution.cost, tour = ?solution.tour, "Christofides solver completed");
}

#[allow(dead_code)]
fn test_kruskal() {
    let (problem_id, data) = read_tsp_file("./data/burma14.tsp");

    let tsp_instance = try_parse(problem_id, data).expect("failed to read instance");
    let problem_instance: TsplibInstance =
        tsp_instance.try_into().expect("failed to convert instance");

    let mst = problem_instance
        .try_get_mst_kruskal()
        .expect("failed to compute MST using Kruskal's algorithm");

    tracing::info!(
        edge_count = mst.edges.len(),
        "Kruskal's algorithm completed"
    );
    mst.edges.iter().for_each(|edge| {
        tracing::info!("Edge: {} - {}, weight: {}", edge.u, edge.v, edge.weight);
    });
}

#[allow(dead_code)]
fn test_prim() {
    let (problem_id, data) = read_tsp_file("./data/gr24.tsp");

    let parse_time = Instant::now();
    let tsp_instance = try_parse(problem_id, data).expect("failed to read instance");
    println!("Parsing instance took {:.2?}", parse_time.elapsed());

    let conversion_time = Instant::now();
    let problem_instance: TsplibInstance =
        tsp_instance.try_into().expect("failed to convert instance");
    println!("Converting instance took {:.2?}", conversion_time.elapsed());

    let mst_time = Instant::now();
    let mst = problem_instance
        .try_get_mst_prim(1)
        .expect("failed to compute MST using Prim's algorithm");
    println!("Computing MST took {:.2?}", mst_time.elapsed());

    tracing::info!(edge_count = mst.edges.len(), "Prim's algorithm completed");
    mst.edges.iter().for_each(|edge| {
        tracing::info!("Edge: {} - {}, weight: {}", edge.u, edge.v, edge.weight);
    });
}

#[allow(dead_code)]
fn test_boruvka() {
    let (problem_id, data) = read_tsp_file("./data/gr24.tsp");

    let parse_time = Instant::now();
    let tsp_instance = try_parse(problem_id, data).expect("failed to read instance");
    println!("Parsing instance took {:.2?}", parse_time.elapsed());

    let conversion_time = Instant::now();
    let problem_instance: TsplibInstance =
        tsp_instance.try_into().expect("failed to convert instance");
    println!("Converting instance took {:.2?}", conversion_time.elapsed());

    let mst_time = Instant::now();
    let mst = problem_instance
        .try_get_mst_boruvka()
        .expect("failed to compute MST using Borůvka's algorithm");
    println!("Computing MST took {:.2?}", mst_time.elapsed());

    tracing::info!(
        edge_count = mst.edges.len(),
        "Borůvka's algorithm completed"
    );
    mst.edges.iter().for_each(|edge| {
        tracing::info!("Edge: {} - {}, weight: {}", edge.u, edge.v, edge.weight);
    });
}
