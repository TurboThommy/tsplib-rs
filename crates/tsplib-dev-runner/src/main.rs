//! This module contains the main function and test functions for parsing TSP files and converting them to graph representations.
use std::time::Instant;

use itertools::Itertools;
use tsplib_core::{
    context::ExecutionContext,
    models::TsplibInstance,
    reader::{read_tsp_file, read_tsp_files},
};
use tsplib_parser::{parse, try_parse};
use tsplib_solver::TspSolver;

/// The main function serves as the entry point of the program, calling the test functions for parsing TSP files.
fn main() {
    let now = Instant::now();
    // test_parse();
    // test_try_parse();
    // test_edge_weight_matrix_conversion();
    // test_graph_conversion();
    // test_instance_to_string();
    // test_greedy_solver();
    // test_held_karp_solver();
    // test_kruskal();
    // test_prim();
    test_boruvka();
    println!("Total execution time: {:.2?}", now.elapsed());
}

/// Tests the `parse` function by reading TSP files from the "./data" directory, parsing them, and printing the results.
#[allow(dead_code)]
fn test_parse() {
    println!("Testing parse()");
    let instances = read_tsp_files("./data")
        .into_iter()
        .map(|(id, data)| parse(id, data))
        .collect::<Vec<_>>();

    println!("Parsed {} instances", instances.len());
}

/// Tests the `try_parse` function by reading TSP files from the "./data" directory, attempting to parse them, and printing the results.
#[allow(dead_code)]
fn test_try_parse() {
    println!("Testing try_parse()");
    let try_instances = read_tsp_files("./data")
        .into_iter()
        .map(|(id, data)| try_parse(id, data))
        .collect::<Vec<_>>();

    println!("Try parsed {} instances", try_instances.len());
    println!(
        "Successfully try parsed {} instances",
        try_instances.iter().filter(|r| r.is_ok()).count()
    );
}

/// Tests the conversion of TSP instances to graph representations and prints the sizes of the resulting graphs.
#[allow(dead_code)]
fn test_graph_conversion() {
    println!("Testing graph conversion and heap size calculation");
    // only use instances with dimension <= 10000
    let tsp_instances = read_tsp_files("./data")
        .into_iter()
        .flat_map(|(id, data)| try_parse(id, data))
        .sorted_by_key(|i| i.dimension)
        .rev()
        .collect::<Vec<_>>();

    println!("Total instances: {}", tsp_instances.len());

    let sizes = tsp_instances
        .iter()
        .flat_map(|i| {
            println!(
                "\nTrying to convert instance {} with dimension {}",
                i.name, i.dimension
            );
            let start = std::time::Instant::now();
            let result = i.try_into().ok();
            let try_into_duration = start.elapsed();
            println!("\ttry_into in {:.2?}", try_into_duration);

            result.map(|instance: TsplibInstance| {
                let start = std::time::Instant::now();
                let size = instance.heap_size();
                let heap_size_duration = start.elapsed();
                println!("\theap size calculation in {:.2?}", heap_size_duration);

                (i.name.clone(), size)
            })
        })
        .collect::<Vec<_>>();

    sizes.iter().for_each(|(name, size)| {
        println!(
            "Instance: {}, size: {:.2} MB",
            name,
            *size as f64 / (1024.0 * 1024.0)
        )
    })
}

/// Tests the conversion of TSP instances with EDGE_WEIGHT_SECTION to adjacency matrices and prints the results.
#[allow(dead_code)]
fn test_edge_weight_matrix_conversion() {
    println!("Testing edge weight matrix conversion");
    let tsp_instances = read_tsp_files("./data/test_data")
        .into_iter()
        .flat_map(|(id, data)| try_parse(id, data))
        .collect::<Vec<_>>();

    let adjacency_matrices = tsp_instances
        .iter()
        .flat_map(|i| i.try_calculate_adjacency_matrix_edge_weights(ExecutionContext::default()));

    adjacency_matrices.for_each(|m| {
        println!("{:?}", m);
    });
}

/// Tests the to_string implementation of TSPInstance by reading a TSP file, parsing it, and printing the resulting string representation.
#[allow(dead_code)]
fn test_instance_to_string() {
    println!("Testing TSPInstance to_string()");
    let (problem_id, data) = read_tsp_file("./data/linhp318.tsp");
    let tsp_instance = try_parse(problem_id, data).expect("failed to read instance");

    println!("{}", tsp_instance);
}

/// Tests the greedy TSP solver by creating a sample problem instance and attempting to solve it, printing the results.
#[allow(dead_code)]
fn test_greedy_solver() {
    println!("Testing Greedy solver");

    let (problem_id, data) = read_tsp_file("./data/burma14.tsp");

    let tsp_instance = try_parse(problem_id, data).expect("failed to read instance");
    let problem_instance: TsplibInstance =
        tsp_instance.try_into().expect("failed to convert instance");

    let solver = tsplib_solver::Greedy {};
    let solution = solver
        .try_solve(&problem_instance, 1)
        .expect("failed to solve instance");

    println!("Tour: {:?}", solution.tour);
    println!("Total distance: {}", solution.cost);
}

#[allow(dead_code)]
fn test_held_karp_solver() {
    println!("Testing Held-Karp solver");

    let (problem_id, data) = read_tsp_file("./data/burma14.tsp");

    let tsp_instance = try_parse(problem_id, data).expect("failed to read instance");
    let problem_instance: TsplibInstance =
        tsp_instance.try_into().expect("failed to convert instance");

    let solver = tsplib_solver::HeldKarp::try_new(25).expect("failed to create HeldKarp solver");
    let solution = solver
        .try_solve(&problem_instance, 1)
        .expect("failed to solve instance");

    println!("Tour: {:?}", solution.tour);
    println!("Total distance: {}", solution.cost);
}

#[allow(dead_code)]
fn test_kruskal() {
    println!("Testing Kruskal's algorithm");

    let (problem_id, data) = read_tsp_file("./data/burma14.tsp");

    let tsp_instance = try_parse(problem_id, data).expect("failed to read instance");
    let problem_instance: TsplibInstance =
        tsp_instance.try_into().expect("failed to convert instance");

    let mst = problem_instance
        .try_get_mst_kruskal()
        .expect("failed to compute MST using Kruskal's algorithm");

    println!("Edges in the MST:");
    mst.edges.iter().for_each(|edge| {
        println!("Edge: {} - {}, weight: {}", edge.u, edge.v, edge.weight);
    });
}

#[allow(dead_code)]
fn test_prim() {
    println!("Testing Prim's algorithm");

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

    println!("Edges in the MST:");
    mst.edges.iter().for_each(|edge| {
        println!("Edge: {} - {}, weight: {}", edge.u, edge.v, edge.weight);
    });
}

#[allow(dead_code)]
fn test_boruvka() {
    println!("Testing Borůvka's algorithm");

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

    println!("Edges in the MST:");
    mst.edges.iter().for_each(|edge| {
        println!("Edge: {} - {}, weight: {}", edge.u, edge.v, edge.weight);
    });
}
