use std::fs;
use tsplib_parser::{parse, try_parse};

/// The main function serves as the entry point of the program, calling the test functions for parsing TSP files.
fn main() {
    test_parse();
    test_try_parse();
}

/// Tests the `parse` function by reading TSP files from the "./data" directory, parsing them, and printing the results.
fn test_parse() {
    let instances = read_tsp_files("./data")
        .into_iter()
        .map(parse)
        .collect::<Vec<_>>();

    println!("Parsed {} instances", instances.len());
    // instances.iter().for_each(|i| println!("{}\n", i));
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
    println!(
        "Failed to try parse {} instances",
        try_instances.iter().filter(|r| r.is_err()).count()
    );
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
