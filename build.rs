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
    if !nsync_path.exists() {
        let mut cmd = Command::new("git");
        cmd.arg("clone")
            .arg("git@github.com:google/nsync.git")
            .arg(&nsync_path);
        if cmd.spawn().is_err() {
            panic!(
                "Failed to clone nsync repository. Ensure git is installed and available in PATH."
            );
        }
    }
    generate_bindings(&nsync_path.join("public"));
    println!("cargo:rustc-link-lib=static=nsync");
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
