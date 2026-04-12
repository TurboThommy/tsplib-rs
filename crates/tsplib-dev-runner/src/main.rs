use std::fs;
use tsplib_parser::parse_tsp_file;
fn main() {
    let files = fs::read_dir("data")
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
        .collect::<Vec<_>>();

    let instances = files
        .into_iter()
        .map(|file| parse_tsp_file(file.path().to_str().unwrap()))
        .collect::<Vec<_>>();

    println!("Parsed {} instances", instances.len());
    instances
        .iter()
        .filter(|i| i.name == "usa13509")
        .for_each(|i| println!("{}\n", i));
}
