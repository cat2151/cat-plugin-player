//! Builds, embeds, and delay-links shim/ (the C++ DLL over uapmd).
//!
//! Environment variables:
//!   UAPMD_DIR        path to the uapmd checkout (default: ../uapmd, beside this package)
//!   UH_GENERATOR     CMake generator (default: "Visual Studio 18 2026")
//!   UH_SKIP_CMAKE=1  do not run CMake; just link what is already built
//!   UH_RECONFIGURE=1 run the CMake configure step again
//!
//! uapmd's dependencies are fetched into CPM_SOURCE_CACHE (target/cpm), as uapmd's
//! README does. Without it, CPM uses the FetchContent_Populate form that ignores
//! FETCHCONTENT_*_DISCONNECTED: every configure updates ImTimeline's branch and
//! re-applies uapmd's patches, and ImTimeline fails as already patched. A package
//! already in the cache is not fetched or patched again. Deleting target/shim keeps
//! target/cpm; a broken cached source needs target/cpm deleted too (`cargo clean`).
//! Moving the cache needs a new build directory: CMakeCache.txt keeps CPM's old
//! location, and configure then fails with "Unknown CMake command CPMAddPackage".
//! CPM warns that the libremidi cache is dirty: that is uapmd's own patch.
//!
//! The configure step runs once per build directory and is then skipped, as it takes
//! over a minute even with the cache.
//!
//! The shim is always built as Release, also for `cargo build` without --release:
//! Rust always links the release CRT, and a Debug C++ build would use the debug one.
//!
//! The C++ side links the CRT the same way as Rust: statically (/MT) when the target
//! feature `crt-static` is enabled (`-C target-feature=+crt-static`), otherwise as DLLs
//! (/MD). The choice is fixed at configure time, so switching it needs a new build
//! directory (delete target/shim); an existing one keeps the CRT it was configured with.
//! See docs/adr/0023-static-crt-release-build.md.

use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Bump to force one new configure run in existing build directories.
const CONFIGURE_STAMP: &str = ".uh_configured_v5";

fn run(cmd: &mut Command) {
    println!("cargo:warning=running: {:?}", cmd);
    let status = cmd
        .status()
        .unwrap_or_else(|e| panic!("could not start {:?}: {e}", cmd));
    if !status.success() {
        panic!("{:?} failed with {status}", cmd);
    }
}

pub fn build() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let player_dir = if manifest_dir.join("shared/host.cmake").exists() {
        manifest_dir.clone()
    } else {
        manifest_dir.join("../cat-plugin-player")
    };
    let shim_dir = player_dir.join("shim");
    println!(
        "cargo:rerun-if-changed={}",
        player_dir
            .join("patches/uapmd/parameter-automation")
            .display()
    );
    let configure_dir = manifest_dir.join("shim");
    for file in ["host.cmake", "smf_shim.cpp", "build.rs"] {
        println!(
            "cargo:rerun-if-changed={}",
            player_dir.join("shared").join(file).display()
        );
    }
    println!(
        "cargo:rerun-if-changed={}",
        configure_dir.join("CMakeLists.txt").display()
    );
    // Deliberately not OUT_DIR: that path is long, and the dependency trees CMake
    // unpacks below it would run into the Windows path length limit.
    let build_dir = manifest_dir.join("target").join("shim");
    let out_dir = build_dir.join("out");
    let uapmd_dir = env::var("UAPMD_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| manifest_dir.join("..").join("uapmd"));
    let uapmd_dir = uapmd_dir.canonicalize().unwrap_or(uapmd_dir);

    for f in [
        "CMakeLists.txt",
        "uapmd_shim.cpp",
        "uapmd_shim.h",
        "ui_thumbnail.h",
        "window_placement.h",
        "owned_instance.h",
        "parameter_binding.h",
        "parameter_api.inc",
        "plugin_catalog.cpp",
        "plugin_catalog.h",
        "windows_vst3_binary.h",
        "shutdown_overlay.cpp",
        "plugin_specific/shu_ui.h",
    ] {
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
    println!(
        "cargo:rerun-if-changed={}",
        player_dir.join("patches/uapmd/clap-note-dialect").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        uapmd_dir.join("source/remidy/src/clap").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        player_dir
            .join("patches/uapmd/clap-state-load-error")
            .display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        player_dir
            .join("patches/uapmd/scan-tool-data-dir-race")
            .display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        uapmd_dir
            .join("source/uapmd-plugin-hosting/src/scanner/PluginScanTool.cpp")
            .display()
    );

    println!(
        "cargo:rerun-if-changed={}",
        player_dir.join("patches/uapmd/stb-revision-pin").display()
    );

    if env::var_os("UH_SKIP_CMAKE").is_none() {
        let generator =
            env::var("UH_GENERATOR").unwrap_or_else(|_| "Visual Studio 18 2026".to_string());
        let stamp = build_dir.join(CONFIGURE_STAMP);
        if !stamp.exists() || env::var_os("UH_RECONFIGURE").is_some() {
            let mut configure = Command::new("cmake");
            configure
                .arg("-S")
                .arg(&configure_dir)
                .arg("-B")
                .arg(&build_dir)
                .arg("-G")
                .arg(&generator)
                .arg(format!("-DUAPMD_DIR={}", cmake_path(&uapmd_dir)))
                .arg("-DFETCHCONTENT_UPDATES_DISCONNECTED=ON")
                .arg(format!(
                    "-DCPM_SOURCE_CACHE={}",
                    cmake_path(&manifest_dir.join("target").join("cpm"))
                ))
                .arg(format!(
                    "-DUH_MSVC_RUNTIME_LIBRARY={}",
                    msvc_runtime_library()
                ));
            if generator.starts_with("Visual Studio") && !build_dir.join("CMakeCache.txt").exists()
            {
                configure.arg("-A").arg("x64");
            } else if !generator.starts_with("Visual Studio") {
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
            .args(["uapmd_shim", "smf_shim"]));
    }

    println!("cargo:rustc-link-search=native={}", out_dir.display());
    println!("cargo:rustc-link-lib=dylib=uapmd_shim");
    println!("cargo:rustc-link-lib=static=smf_shim");
    println!(
        "cargo:rustc-link-search=native={}",
        build_dir.join("Release/lib/static").display()
    );
    println!("cargo:rustc-link-lib=static=umppi");
    // cargo install only deploys the exe. Embed the DLL and load it before the
    // first FFI call, so CLI commands and installed binaries need no loose DLL.
    println!("cargo:rustc-link-lib=delayimp");
    println!("cargo:rustc-link-arg=/DELAYLOAD:uapmd_shim.dll");
    println!(
        "cargo:rustc-env=UAPMD_SHIM_DLL={}",
        out_dir.join("uapmd_shim.dll").display()
    );
}

/// The CMake MSVC runtime that matches how Rust links the CRT; empty keeps uapmd's choice (DLL).
fn msvc_runtime_library() -> &'static str {
    let features = env::var("CARGO_CFG_TARGET_FEATURE").unwrap_or_default();
    if features.split(',').any(|f| f == "crt-static") {
        "MultiThreaded"
    } else {
        ""
    }
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
