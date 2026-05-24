//! This module provides functions for reading .tsp files from a specified directory.
//! It includes functionality to read all .tsp files in a directory as well as to read a single .tsp file, returning the problem ID (derived from the filename) and the file content as a string.
use std::{fs, path::Path};

use crate::enums::IoError;

/// Reads all .tsp files from the provided path directory.
///
/// # Arguments
/// * `path` - The directory path from which to read .tsp files.
///
/// # Returns
/// * `Result<Vec<(String, String)>, IoError>` - A vector of tuples, where each tuple contains the problem ID (filename without extension) and the file content as a string,
///   or an error if the directory cannot be read or if any file cannot be read.
pub fn try_read_tsp_files(path: &str) -> Result<Vec<(String, String)>, IoError> {
    let files = fs::read_dir(path)
        .map_err(|e| IoError::DirectoryReadError(e.to_string()))?
        .map(|entry| entry.map_err(|e| IoError::DirectoryEntryReadError(e.to_string())))
        .collect::<Result<Vec<_>, IoError>>()?
        .iter()
        .filter(|entry| entry.path().is_file())
        .filter(|entry| match entry.path().extension() {
            Some(ext) => ext == "tsp",
            None => false,
        })
        .map(|entry| {
            // extract the problem ID from the filename (without extension)
            let problem_id = entry
                .path()
                .file_stem()
                .ok_or(IoError::InvalidFileStem(
                    entry.path().to_string_lossy().to_string(),
                ))?
                .to_string_lossy()
                .to_string();

            // read the file content as a string
            let data = try_read_file(entry.path().to_str().ok_or(IoError::InvalidFilePath(
                entry.path().to_string_lossy().to_string(),
            ))?)?;

            Ok((problem_id, data))
        })
        .collect::<Result<Vec<_>, IoError>>()?;

    Ok(files)
}

/// Reads a single .tsp file from the provided path and returns its problem ID and content as a string.
///
/// # Arguments
/// * `path` - The file path of the .tsp file to read.
///
/// # Returns
/// * `Result<(String, String), IoError>` - A tuple containing the problem ID (filename without extension) and the file content as a string,
///   or an error if the file cannot be read or if the filename does not contain a valid stem.
pub fn try_read_tsp_file(path: &str) -> Result<(String, String), IoError> {
    let problem_id = Path::new(path)
        .file_stem()
        .ok_or(IoError::InvalidFileStem(path.to_string()))?
        .to_string_lossy()
        .to_string();

    let data = try_read_file(path)?;
    Ok((problem_id, data))
}

/// Reads the contents of a file at the given path and returns it as a string.
///
/// # Arguments
/// * `file_path` - The file path of the file to read.
///
/// # Returns
/// * `Result<String, IoError>` - The content of the file as a string or an error if the file cannot be read.
fn try_read_file(file_path: &str) -> Result<String, IoError> {
    let file_content = fs::read_to_string(file_path);

    match file_content {
        Ok(content) => Ok(content),
        Err(e) => Err(IoError::FileReadError(e.to_string())),
    }
}

/// Convenience function for try_read_tsp_files that panics on error.
///
/// # Arguments
/// * `path` - The directory path from which to read .tsp files.
///
/// # Returns
/// * `Vec<(String, String)>` - A vector of tuples, where each tuple contains the problem ID (filename without extension) and the file content as a string.
pub fn read_tsp_files(path: &str) -> Vec<(String, String)> {
    try_read_tsp_files(path).expect("Failed to read all TSP files from directory")
}

/// Convenience function for try_read_tsp_file that panics on error.
///
/// # Arguments
/// * `path` - The file path of the .tsp file to read.
///
/// # Returns
/// * `(String, String)` - A tuple containing the problem ID (filename without extension) and the file content as a string.
pub fn read_tsp_file(path: &str) -> (String, String) {
    try_read_tsp_file(path).expect("Failed to read TSP file")
}
