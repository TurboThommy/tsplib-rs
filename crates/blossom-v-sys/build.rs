//! This crate provides FFI bindings to the Blossom V library for solving the minimum weight perfect matching problem.

use std::{env, path::PathBuf};

/// Build script for the blossom-v-sys crate, which compiles the C++ bridge and the Blossom V source files into a static library.
///
/// # Requirements
/// * The `BLOSSOM_V_PATH` environment variable must be set to the path of the Blossom V library source code,
///   which can be obtained from https://pub.ist.ac.at/~vnk/software/blossom5-v2.05.src.tar.gz.
fn main() {
    // get the path to the blossom v library from the environment variable
    let blossom_v_path = PathBuf::from(env::var("BLOSSOM_V_PATH").expect(
        "BLOSSOM_V_PATH environment variable must be set to the path of the blossom v library",
    ));

    // tell cargo to rerun this build script if the environment variable changes or if any of the relevant source files change
    println!("cargo:rerun-if-env-changed=BLOSSOM_V_PATH");
    println!("cargo:rerun-if-changed=cpp/blossom_bridge.cpp");
    for file in [
        "PerfectMatching.h",
        "PMimplementation.h",
        "LCA.h",
        "PQ.h",
        "block.h",
        "timer.h",
        "PMduals.cpp",
        "PMexpand.cpp",
        "PMinit.cpp",
        "PMinterface.cpp",
        "PMmain.cpp",
        "PMrepair.cpp",
        "PMshrink.cpp",
        "misc.cpp",
        "MinCost/MinCost.cpp",
        "MinCost/MinCost.h",
        "MinCost/instances.inc",
    ] {
        println!(
            "cargo:rerun-if-changed={}",
            blossom_v_path.join(file).display()
        );
    }

    // compile the C++ bridge and the Blossom V source files into a static library
    cc::Build::new()
        .cpp(true)
        .file("cpp/blossom_bridge.cpp")
        .file(blossom_v_path.join("PMduals.cpp"))
        .file(blossom_v_path.join("PMexpand.cpp"))
        .file(blossom_v_path.join("PMinit.cpp"))
        .file(blossom_v_path.join("PMinterface.cpp"))
        .file(blossom_v_path.join("PMmain.cpp"))
        .file(blossom_v_path.join("PMrepair.cpp"))
        .file(blossom_v_path.join("PMshrink.cpp"))
        .file(blossom_v_path.join("misc.cpp"))
        .file(blossom_v_path.join("MinCost/MinCost.cpp"))
        .include(&blossom_v_path)
        .include(blossom_v_path.join("MinCost"))
        .compile("blossom_v_bridge");
}
