//! This module contains the main function and test functions for parsing TSP files and converting them to graph representations.
use itertools::Itertools;
use std::fs;
use tsplib_core::models::ProblemInstance;
use tsplib_parser::{parse, try_parse};
use tsplib_solver::TspSolver;

/// The main function serves as the entry point of the program, calling the test functions for parsing TSP files.
fn main() {
    // test_parse();
    // test_try_parse();
    // test_edge_weight_matrix_conversion();
    // test_graph_conversion();
    // test_instance_to_string();
    // test_greedy_solver();
    test_held_carp_solver();
}

/// Tests the `parse` function by reading TSP files from the "./data" directory, parsing them, and printing the results.
#[allow(dead_code)]
fn test_parse() {
    let instances = read_tsp_files("./data")
        .into_iter()
        .map(parse)
        .collect::<Vec<_>>();

    println!("Parsed {} instances", instances.len());
}

/// Tests the `try_parse` function by reading TSP files from the "./data" directory, attempting to parse them, and printing the results.
#[allow(dead_code)]
fn test_try_parse() {
    let try_instances = read_tsp_files("./data")
        .into_iter()
        .map(try_parse)
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
    // only use instances with dimension <= 10000
    let tsp_instances = read_tsp_files("./data")
        .into_iter()
        .flat_map(try_parse)
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

            result.map(|instance: ProblemInstance| {
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
    let tsp_instances = read_tsp_files("./data/test_data")
        .into_iter()
        .flat_map(try_parse)
        .collect::<Vec<_>>();

    let adjacency_matrices = tsp_instances
        .iter()
        .flat_map(|i| i.try_calculate_adjacency_matrix_edge_weights());

    adjacency_matrices.for_each(|m| {
        println!("{:?}", m);
    });
}

/// Tests the to_string implementation of TSPInstance by reading a TSP file, parsing it, and printing the resulting string representation.
#[allow(dead_code)]
fn test_instance_to_string() {
    let tsp_instance =
        try_parse(read_file("./data/linhp318.tsp")).expect("failed to read instance");

    println!("{}", tsp_instance);
}

/// Tests the greedy TSP solver by creating a sample problem instance and attempting to solve it, printing the results.
#[allow(dead_code)]
fn test_greedy_solver() {
    let tsp_instance = try_parse(read_file("./data/burma14.tsp")).expect("failed to read instance");
    let problem_instance: ProblemInstance =
        tsp_instance.try_into().expect("failed to convert instance");

    let solver = tsplib_solver::Greedy {};
    let solution = solver
        .try_solve(&problem_instance, 1)
        .expect("failed to solve instance");

    println!("Tour: {:?}", solution.tour);
    println!("Total distance: {}", solution.cost);
}

#[allow(dead_code)]
fn test_held_carp_solver() {
    let tsp_instance = try_parse(read_file("./data/burma14.tsp")).expect("failed to read instance");
    let problem_instance: ProblemInstance =
        tsp_instance.try_into().expect("failed to convert instance");

    let solver = tsplib_solver::HeldCarp { max_dimension: 20 };
    let solution = solver
        .try_solve(&problem_instance, 1)
        .expect("failed to solve instance");

    println!("Tour: {:?}", solution.tour);
    println!("Total distance: {}", solution.cost);

    println!("Adjacency matrix:");
    for row in problem_instance.adjacency_matrix.iter() {
        println!("{:?}", row);
    }
}

/// Reads all .tsp files from the provided path directory and returns their contents as a vector of strings.
fn read_tsp_files(path: &str) -> Vec<String> {
    fs::read_dir(path)
        .expect("Unable to read directory")
        .filter(|entry| {
            entry
                .as_ref()
                .expect("Unable to read entry")
                .path()
                .is_file()
        })
        .map(|entry| entry.expect("Unable to read entry"))
        .filter(|entry| match entry.path().extension() {
            Some(ext) => ext == "tsp",
            None => false,
        })
        .map(|entry| read_file(entry.path().to_str().unwrap()))
        .collect::<Vec<_>>()
}

/// Reads the contents of a file at the given path and returns it as a string.
fn read_file(file_path: &str) -> String {
    fs::read_to_string(file_path).expect("Unable to read file")
}
