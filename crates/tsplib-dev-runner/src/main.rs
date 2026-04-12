use std::fs;
use tsplib_parser::{parse, try_parse};
fn main() {
    test_parse();
    test_try_parse();
}

fn test_parse() {
    let instances = read_tsp_files().into_iter().map(parse).collect::<Vec<_>>();

    println!("Parsed {} instances", instances.len());
    // instances.iter().for_each(|i| println!("{}\n", i));
}

fn test_try_parse() {
    let try_instances = read_tsp_files()
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

fn read_tsp_files() -> Vec<String> {
    fs::read_dir("data")
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

fn read_file(file_path: &str) -> String {
    fs::read_to_string(file_path).expect("Unable to read file")
}
