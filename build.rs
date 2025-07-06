extern crate bindgen;

use std::env;
use std::path::Path;
use std::process::Command;

fn main() {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();

    if let Ok(lib) = pkg_config::probe_library("nsync") {
        println!("Found system nsync installation");
        generate_bindings(&lib.include_paths[0]);
        return;
    }

    println!("System nsync not found, building from source...");

    let nsync_path = Path::new(&manifest_dir).join("vendored/nsync");
    let nsync_build = nsync_path.join("build");

    // Ensure vendored/nsync is cloned
    if !nsync_path.exists() {
        let mut cmd = Command::new("git");
        cmd.arg("clone")
            .arg("git@github.com:google/nsync.git")
            .arg(&nsync_path);
        if cmd.status().unwrap().success() == false {
            panic!(
                "Failed to clone nsync repository. Ensure git is installed and available in PATH."
            );
        }
    }

    // Run CMake to build the library
    if !nsync_build.exists() {
        std::fs::create_dir_all(&nsync_build).expect("Failed to create build directory");
    }

    let status = Command::new("cmake")
        .arg("..")
        .current_dir(&nsync_build)
        .status()
        .expect("Failed to run cmake");
    if !status.success() {
        panic!("CMake configuration failed");
    }

    let status = Command::new("make")
        .current_dir(&nsync_build)
        .status()
        .expect("Failed to run make");
    if !status.success() {
        panic!("Failed to build nsync using make");
    }

    // Link to the built static library
    println!("cargo:rustc-link-search=native={}", nsync_build.display());
    println!("cargo:rustc-link-lib=static=nsync");

    // Link to threading libraries
    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap();
    match target_os.as_str() {
        "linux" | "macos" => {
            println!("cargo:rustc-link-lib=pthread");
        }
        "windows" => {
            println!("cargo:rustc-link-lib=kernel32");
            println!("cargo:rustc-link-lib=synchronization");
        }
        _ => {}
    }

    // Generate bindings from the public headers
    generate_bindings(&nsync_path.join("public"));

    println!("cargo:rerun-if-changed=vendored/nsync");
}

fn generate_bindings(include_dir: &Path) {
    let bindings = bindgen::Builder::default()
        .header(include_dir.join("nsync.h").to_str().unwrap())
        .header(include_dir.join("nsync_atomic.h").to_str().unwrap())
        .header(include_dir.join("nsync_counter.h").to_str().unwrap())
        .header(include_dir.join("nsync_cv.h").to_str().unwrap())
        .header(include_dir.join("nsync_debug.h").to_str().unwrap())
        .header(include_dir.join("nsync_mu.h").to_str().unwrap())
        .header(include_dir.join("nsync_mu_wait.h").to_str().unwrap())
        .header(include_dir.join("nsync_note.h").to_str().unwrap())
        .header(include_dir.join("nsync_once.h").to_str().unwrap())
        .header(include_dir.join("nsync_time.h").to_str().unwrap())
        .header(include_dir.join("nsync_waiter.h").to_str().unwrap())
        .clang_arg(format!("-I{}", include_dir.display()))
        .allowlist_type("nsync_.*")
        .allowlist_function("nsync_.*")
        .allowlist_var("nsync_.*")
        .ctypes_prefix("::core::ffi")
        .generate()
        .expect("Unable to generate bindings");

    bindings
        .write_to_file("src/bindings.rs")
        .expect("Couldn't write bindings!");
}
