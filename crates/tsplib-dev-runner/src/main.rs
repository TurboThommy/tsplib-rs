use itertools::Itertools;
use std::fs;
use tsplib_core::models::ProblemInstance;
use tsplib_parser::{parse, try_parse};

/// The main function serves as the entry point of the program, calling the test functions for parsing TSP files.
fn main() {
    test_parse();
    test_try_parse();
    test_edge_weight_matrix_conversion();
    test_graph_conversion();
}

/// Tests the `parse` function by reading TSP files from the "./data" directory, parsing them, and printing the results.
fn test_parse() {
    let instances = read_tsp_files("./data")
        .into_iter()
        .map(parse)
        .collect::<Vec<_>>();

    println!("Parsed {} instances", instances.len());
}

/// Tests the `try_parse` function by reading TSP files from the "./data" directory, attempting to parse them, and printing the results.
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

fn test_graph_conversion() {
    // only use instances with dimension <= 10000
    let tsp_instances = read_tsp_files("./data")
        .into_iter()
        .flat_map(try_parse)
        .filter(|r| r.dimension <= 20000)
        .collect::<Vec<_>>();
    println!(
        "Found {} TSP instances with dimension <= 20000",
        tsp_instances.len()
    );

    let problem_instances: Vec<ProblemInstance> = tsp_instances
        .iter()
        .flat_map(|i| {
            println!("Converting instance {} to graph representation...", i.name);
            i.try_into()
        })
        .collect::<Vec<_>>();

    println!(
        "Successfully converted {} TSP instances to graph representations",
        problem_instances.len()
    );

    problem_instances
        .iter()
        .map(|instance| (&instance.name, instance.heap_size()))
        .sorted_by_key(|(_, size)| *size)
        .rev()
        .for_each(|(name, size)| {
            println!(
                "Instance {}, size: {:.2} MB",
                name,
                size as f64 / (1024.0 * 1024.0)
            );
        });
}

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
