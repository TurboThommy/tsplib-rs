use std::{env, path::PathBuf};

fn main() {
    let blossom_v_path = PathBuf::from(env::var("BLOSSOM_V_PATH").expect(
        "BLOSSOM_V_PATH environment variable must be set to the path of the blossom v library",
    ));

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
