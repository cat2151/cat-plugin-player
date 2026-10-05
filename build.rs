//! Builds shim/ (the C++ DLL over uapmd) with CMake and links it.
//!
//! Environment variables:
//!   UAPMD_DIR        path to the uapmd checkout (default: ../uapmd, beside this package)
//!   UH_GENERATOR     CMake generator (default: "Visual Studio 17 2022")
//!   UH_SKIP_CMAKE=1  do not run CMake; just link what is already built
//!   UH_RECONFIGURE=1 run the CMake configure step again
//!
//! The configure step runs once per build directory and is then skipped: uapmd pulls
//! a patched dependency from a branch (ImTimeline), and configuring a second time
//! tries to update and re-patch it, which fails. `cmake --build` still re-runs
//! configure by itself when a CMakeLists.txt changes; FETCHCONTENT_UPDATES_DISCONNECTED
//! keeps that from touching dependencies that are already downloaded.
//!
//! The shim is always built as Release, also for `cargo build` without --release:
//! Rust always links the release CRT, and a Debug C++ build would use the debug one.

use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Bump to force one new configure run in existing build directories.
const CONFIGURE_STAMP: &str = ".uh_configured_v2";

fn run(cmd: &mut Command) {
    println!("cargo:warning=running: {:?}", cmd);
    let status = cmd
        .status()
        .unwrap_or_else(|e| panic!("could not start {:?}: {e}", cmd));
    if !status.success() {
        panic!("{:?} failed with {status}", cmd);
    }
}

fn main() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let shim_dir = manifest_dir.join("shim");
    // Deliberately not OUT_DIR: that path is long, and the dependency trees CMake
    // unpacks below it would run into the Windows path length limit.
    let build_dir = manifest_dir.join("target").join("shim");
    let out_dir = build_dir.join("out");

    for f in ["CMakeLists.txt", "uapmd_shim.cpp", "uapmd_shim.h"] {
        println!("cargo:rerun-if-changed={}", shim_dir.join(f).display());
    }
    for v in [
        "UAPMD_DIR",
        "UH_GENERATOR",
        "UH_SKIP_CMAKE",
        "UH_RECONFIGURE",
    ] {
        println!("cargo:rerun-if-env-changed={v}");
    }

    if env::var_os("UH_SKIP_CMAKE").is_none() {
        let generator =
            env::var("UH_GENERATOR").unwrap_or_else(|_| "Visual Studio 17 2022".to_string());
        let uapmd_dir = env::var("UAPMD_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| manifest_dir.join("..").join("uapmd"));

        let stamp = build_dir.join(CONFIGURE_STAMP);
        if !stamp.exists() || env::var_os("UH_RECONFIGURE").is_some() {
            let mut configure = Command::new("cmake");
            configure
                .arg("-S")
                .arg(&shim_dir)
                .arg("-B")
                .arg(&build_dir)
                .arg("-G")
                .arg(&generator)
                .arg(format!("-DUAPMD_DIR={}", cmake_path(&uapmd_dir)))
                .arg("-DFETCHCONTENT_UPDATES_DISCONNECTED=ON");
            if generator.starts_with("Visual Studio") {
                configure.arg("-A").arg("x64");
            } else {
                configure.arg("-DCMAKE_BUILD_TYPE=Release");
            }
            run(&mut configure);
            std::fs::write(&stamp, "").expect("could not write the configure stamp");
        }

        run(Command::new("cmake")
            .arg("--build")
            .arg(&build_dir)
            .arg("--config")
            .arg("Release")
            .arg("--target")
            .arg("uapmd_shim"));
    }

    println!("cargo:rustc-link-search=native={}", out_dir.display());
    println!("cargo:rustc-link-lib=dylib=uapmd_shim");

    // The exe needs the DLL beside it. OUT_DIR is target/<profile>/build/<pkg>-<hash>/out.
    let dll = out_dir.join("uapmd_shim.dll");
    if dll.exists() {
        let profile_dir = PathBuf::from(env::var("OUT_DIR").unwrap())
            .ancestors()
            .nth(3)
            .map(Path::to_path_buf)
            .expect("unexpected OUT_DIR layout");
        std::fs::copy(&dll, profile_dir.join("uapmd_shim.dll"))
            .expect("could not copy uapmd_shim.dll next to the exe");
    } else {
        println!("cargo:warning={} not found", dll.display());
    }
}

/// CMake wants forward slashes.
fn cmake_path(p: &Path) -> String {
    p.display().to_string().replace('\\', "/")
}
