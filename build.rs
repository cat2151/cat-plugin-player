//! Builds, embeds, and delay-links shim/ (the C++ DLL over uapmd).
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

mod build_metadata;

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
    build_metadata::emit(&manifest_dir);
    println!("cargo:rerun-if-changed=build_metadata.rs");
    let shim_dir = manifest_dir.join("shim");
    // Deliberately not OUT_DIR: that path is long, and the dependency trees CMake
    // unpacks below it would run into the Windows path length limit.
    let build_dir = manifest_dir.join("target").join("shim");
    let out_dir = build_dir.join("out");
    let uapmd_dir = env::var("UAPMD_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| manifest_dir.join("..").join("uapmd"));
    let uapmd_dir = uapmd_dir.canonicalize().unwrap_or(uapmd_dir);
    println!("cargo:rustc-env=BUILD_UAPMD_DIR={}", uapmd_dir.display());

    for f in [
        "CMakeLists.txt",
        "uapmd_shim.cpp",
        "uapmd_shim.h",
        "ui_thumbnail.h",
        "window_placement.h",
        "owned_instance.h",
        "plugin_catalog.cpp",
        "plugin_catalog.h",
        "plugin_specific/shu_ui.h",
    ] {
        println!("cargo:rerun-if-changed={}", shim_dir.join(f).display());
    }
    println!(
        "cargo:rerun-if-changed={}",
        manifest_dir
            .join("patches/uapmd/windows-vst3-loader")
            .display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        uapmd_dir
            .join("source/remidy/src/vst3/ClassModuleInfo.cpp")
            .display()
    );
    for v in [
        "UAPMD_DIR",
        "UH_GENERATOR",
        "UH_SKIP_CMAKE",
        "UH_RECONFIGURE",
    ] {
        println!("cargo:rerun-if-env-changed={v}");
    }
    println!(
        "cargo:rerun-if-changed={}",
        manifest_dir
            .join("patches/uapmd/clap-note-dialect")
            .display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        uapmd_dir.join("source/remidy/src/clap").display()
    );

    if env::var_os("UH_SKIP_CMAKE").is_none() {
        let generator =
            env::var("UH_GENERATOR").unwrap_or_else(|_| "Visual Studio 17 2022".to_string());
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
    // cargo install only deploys the exe. Embed the DLL and load it before the
    // first FFI call, so CLI commands and installed binaries need no loose DLL.
    println!("cargo:rustc-link-lib=delayimp");
    println!("cargo:rustc-link-arg=/DELAYLOAD:uapmd_shim.dll");
    println!(
        "cargo:rustc-env=UAPMD_SHIM_DLL={}",
        out_dir.join("uapmd_shim.dll").display()
    );
}

/// CMake wants forward slashes and ordinary Windows paths, not verbatim paths
/// returned by canonicalize (//?/ is not a valid CMake drive prefix).
fn cmake_path(p: &Path) -> String {
    let path = p.display().to_string().replace('\\', "/");
    if let Some(unc) = path.strip_prefix("//?/UNC/") {
        format!("//{unc}")
    } else {
        path.strip_prefix("//?/").unwrap_or(&path).to_owned()
    }
}
