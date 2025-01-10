use anyhow::{bail, Error, Result};
use std::{env, path::PathBuf};

const DEPLOYMENT_TARGET_VAR: &str = "MACOSX_DEPLOYMENT_TARGET";

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

    pub(super) fn get_build_paths() -> Result<(Vec<PathBuf>, Vec<PathBuf>), Error> {
        let mut include_paths = vec![
            out_dir().join("include"),
            out_dir().join("include").join("webrtc-audio-processing-1"),
        ];
        let mut lib_paths = vec![out_dir().join("lib")];

        // If abseil package is installed locally, meson would have picked it for building
        // webrtc-audio-processing-1. Use the same resource for ourselves, too.
        if let Ok(mut lib) = pkg_config::Config::new().probe("absl_base") {
            include_paths.append(&mut lib.include_paths);
            lib_paths.append(&mut lib.link_paths);
        }

        Ok((include_paths, lib_paths))
    }

    pub(super) fn build_if_necessary() -> Result<(), Error> {
        use anyhow::Context;

        if Path::new(BUNDLED_SOURCE_PATH).read_dir()?.next().is_none() {
            eprintln!("The webrtc-audio-processing source directory is empty.");
            eprintln!("See the crate README for installation instructions.");
            eprintln!("Remember to clone the repo recursively if building from source.");
            bail!("Aborting compilation because bundled source directory is empty.");
        }

        let build_dir = out_dir();
        let install_dir = out_dir();

        let webrtc_build_dir = build_dir.join(BUNDLED_SOURCE_PATH);
        let mut meson = Command::new("meson");
        let status = meson
            .args(&["--prefix", install_dir.to_str().unwrap()])
            .arg("-Ddefault_library=static")
            .arg(BUNDLED_SOURCE_PATH)
            .arg(webrtc_build_dir.to_str().unwrap())
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

    let mut cc_build = cc::Build::new();

    // set mac minimum version
    if cfg!(target_os = "macos") {
        let min_version = match env::var(DEPLOYMENT_TARGET_VAR) {
            Ok(ver) => ver,
            Err(_) => {
                String::from(match std::env::var("CARGO_CFG_TARGET_ARCH").unwrap().as_str() {
                    "x86_64" => "10.10", // Using what I found here https://github.com/webrtc-uwp/chromium-build/blob/master/config/mac/mac_sdk.gni#L17
                    "aarch64" => "11.0", // Apple silicon started here.
                    arch => panic!("unknown arch: {}", arch),
                })
            },
        };

        // `cc` doesn't try to pick up on this automatically, but `clang` needs it to
        // generate a "correct" Objective-C symbol table which better matches XCode.
        // See https://github.com/h4llow3En/mac-notification-sys/issues/45.
        cc_build.flag(&format!("-mmacos-version-min={}", min_version));
    }

    cc_build
        .cpp(true)
        .file("src/wrapper.cpp")
        .includes(&include_dirs)
        .flag("-std=c++17")
        .flag("-Wno-unused-parameter")
        .flag("-Wno-deprecated-declarations")
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
