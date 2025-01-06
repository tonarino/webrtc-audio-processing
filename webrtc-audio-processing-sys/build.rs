use anyhow::{bail, Context, Error, Result};
use std::path::PathBuf;

fn out_dir() -> PathBuf {
    std::env::var("OUT_DIR").expect("OUT_DIR environment var not set.").into()
}

#[cfg(not(feature = "bundled"))]
mod webrtc {
    use super::*;

    const LIB_NAME: &str = "webrtc-audio-processing-1";
    const LIB_MIN_VERSION: &str = "1.0";

    pub(super) fn get_build_paths() -> Result<(Vec<PathBuf>, Vec<PathBuf>), Error> {
        let (pkgconfig_include_path, pkgconfig_lib_path) = find_pkgconfig_paths()?;

        let include_path = std::env::var("WEBRTC_AUDIO_PROCESSING_INCLUDE")
            .ok()
            .map(|x| x.into())
            .or(pkgconfig_include_path);
        let lib_path = std::env::var("WEBRTC_AUDIO_PROCESSING_LIB")
            .ok()
            .map(|x| x.into())
            .or(pkgconfig_lib_path);

        match (include_path, lib_path) {
            (Some(include_path), Some(lib_path)) => Ok((vec![include_path], vec![lib_path])),
            _ => {
                eprintln!(
                    "Couldn't find either header or lib files for {}>={}.",
                    LIB_NAME, LIB_MIN_VERSION
                );
                eprintln!("See the crate README for installation instructions, or use the 'bundled' feature to statically compile.");
                bail!("Aborting compilation due to linker failure.");
            },
        }
    }

    pub(super) fn build_if_necessary() -> Result<(), Error> {
        Ok(())
    }

    fn find_pkgconfig_paths() -> Result<(Option<PathBuf>, Option<PathBuf>), Error> {
        Ok(pkg_config::Config::new()
            .atleast_version(LIB_MIN_VERSION)
            .probe(LIB_NAME)
            .and_then(|mut lib| Ok((lib.include_paths.pop(), lib.link_paths.pop())))?)
    }
}

#[cfg(feature = "bundled")]
mod webrtc {
    use super::*;
    use std::{path::Path, process::Command};

    const BUNDLED_SOURCE_PATH: &str = "./webrtc-audio-processing";
    const ABSEIL_SOURCE_PATH: &str = "./abseil-cpp";

    pub(super) fn get_build_paths() -> Result<(Vec<PathBuf>, Vec<PathBuf>), Error> {
        let include_path = out_dir().join("include");
        let lib_path = out_dir().join("lib");
        Ok((vec![include_path.join("webrtc-audio-processing-1"), include_path], vec![lib_path]))
    }

    pub(super) fn build_if_necessary() -> Result<(), Error> {
        if Path::new(BUNDLED_SOURCE_PATH).read_dir()?.next().is_none() {
            eprintln!("The webrtc-audio-processing source directory is empty.");
            eprintln!("See the crate README for installation instructions.");
            eprintln!("Remember to clone the repo recursively if building from source.");
            bail!("Aborting compilation because bundled source directory is empty.");
        }

        let build_dir = out_dir();
        let install_dir = out_dir();
        let webrtc_build_dir = build_dir.join(BUNDLED_SOURCE_PATH);

        // Build Abseil
        let abseil_dir = Path::new(ABSEIL_SOURCE_PATH);
        if abseil_dir.exists() {
            let mut cmake = Command::new("cmake");
            let status = cmake
                .current_dir(&abseil_dir)
                .arg("-B")
                .arg(build_dir.join("abseil-build"))
                .arg("-DCMAKE_INSTALL_PREFIX=".to_owned() + install_dir.to_str().unwrap())
                .arg("-DABSL_PROPAGATE_CXX_STD=ON")
                .status()
                .context("Failed to configure Abseil with CMake")?;
            assert!(status.success(), "Failed to configure Abseil");

            let status = Command::new("cmake")
                .args(&["--build", build_dir.join("abseil-build").to_str().unwrap()])
                .arg("--target")
                .arg("install")
                .status()
                .context("Failed to build Abseil")?;
            assert!(status.success(), "Failed to build Abseil");
        } else {
            eprintln!("Warning: Abseil directory not found at {}", abseil_dir.display());
            eprintln!("Make sure you have cloned the repository with submodules");
            bail!("Missing Abseil dependency");
        }

        // Build WebRTC audio processing
        let mut meson = Command::new("meson");
        let status = meson
            .args(&["--prefix", install_dir.to_str().unwrap()])
            .arg("-Ddefault_library=static")
            .arg(BUNDLED_SOURCE_PATH)
            .arg(webrtc_build_dir.to_str().unwrap())
            .env("PKG_CONFIG_PATH", format!("{}/lib/pkgconfig", install_dir.display()))
            .env("CXXFLAGS", "-I".to_owned() + install_dir.join("include").to_str().unwrap())
            .status()
            .context("Failed to execute meson. Do you have it installed?")?;
        assert!(status.success(), "Command failed: {:?}", &meson);

        let mut ninja = Command::new("ninja");
        let status = ninja
            .args(&["-C", webrtc_build_dir.to_str().unwrap()])
            .arg("install")
            .status()
            .context("Failed to execute ninja. Do you have it installed?")?;
        assert!(status.success(), "Command failed: {:?}", &ninja);

        Ok(())
    }
}

fn main() -> Result<(), Error> {
    webrtc::build_if_necessary()?;
    let (include_dirs, lib_dirs) = webrtc::get_build_paths()?;

    for dir in &lib_dirs {
        println!("cargo:rustc-link-search=native={}", dir.display());
    }

    if cfg!(feature = "bundled") {
        println!("cargo:rustc-link-lib=static=webrtc-audio-processing-1");

        // Add Abseil library linkage & options
        println!("cargo:rustc-link-lib=static=absl_bad_optional_access");
        println!("cargo:rustc-link-lib=static=absl_base");
        println!("cargo:rustc-link-lib=static=absl_throw_delegate");
    } else {
        println!("cargo:rustc-link-lib=dylib=webrtc-audio-processing-1");
    }

    if cfg!(target_os = "macos") {
        // TODO: Remove after confirming this is not necessary.
        //println!("cargo:rustc-link-lib=dylib=c++");
        println!("cargo:rustc-link-lib=framework=CoreFoundation");
    } else {
        // TODO: Remove after confirming this is not necessary.
        //println!("cargo:rustc-link-lib=dylib=stdc++");
    }

    cc::Build::new()
        .cpp(true)
        .file("src/wrapper.cpp")
        .flag("-std=c++17")
        .flag("-Wno-unused-parameter")
        .includes(&include_dirs)
        .out_dir(&out_dir())
        .compile("webrtc_audio_processing_wrapper");

    println!("cargo:rustc-link-lib=static=webrtc_audio_processing_wrapper");

    let binding_file = out_dir().join("bindings.rs");
    let mut builder = bindgen::Builder::default()
        .header("src/wrapper.hpp")
        .clang_args(&["-x", "c++", "-std=c++17", "-fparse-all-comments"])
        .generate_comments(true)
        .enable_cxx_namespaces()
        .allowlist_type("webrtc::AudioProcessing_Error")
        .allowlist_type("webrtc::AudioProcessing_Config")
        .allowlist_type("webrtc::AudioProcessing_RealtimeSetting")
        .allowlist_type("webrtc::StreamConfig")
        .allowlist_type("webrtc::ProcessingConfig")
        .allowlist_function("webrtc_audio_processing_wrapper::.*")
        // The functions returns std::string, and is not FFI-safe.
        .blocklist_item("webrtc::AudioProcessing_Config_ToString")
        .opaque_type("std::.*")
        .derive_debug(true)
        .derive_default(true);
    for dir in &include_dirs {
        builder = builder.clang_arg(&format!("-I{}", dir.display()));
    }
    builder
        .generate()
        .expect("Unable to generate bindings")
        .write_to_file(&binding_file)
        .expect("Couldn't write bindings!");

    Ok(())
}
