use failure::Error;
use std::path::PathBuf;

fn out_dir() -> PathBuf {
    std::env::var("OUT_DIR").expect("OUT_DIR environment var not set.").into()
}

#[cfg(not(feature = "bundled"))]
mod webrtc {
    use super::*;
    use failure::bail;

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
    use failure::bail;
    use std::{path::Path, process::Command};

    const BUNDLED_SOURCE_PATH: &str = "./webrtc-audio-processing";
    const BUNDLED_SOURCE_PATH_ABSEIL: &str = "./abseil-cpp";

    pub(super) fn get_build_paths() -> Result<(Vec<PathBuf>, Vec<PathBuf>), Error> {
        let include_path = out_dir().join("include");
        let lib_path = out_dir().join("lib");
        Ok((vec![include_path.join("webrtc-audio-processing-1"), include_path], vec![lib_path]))
    }

    pub(super) fn build_if_necessary() -> Result<(), Error> {
        if Path::new(BUNDLED_SOURCE_PATH).read_dir()?.next().is_none()
            || Path::new(BUNDLED_SOURCE_PATH_ABSEIL).read_dir()?.next().is_none()
        {
            eprintln!("The webrtc-audio-processing or abseil-cpp source directory is empty.");
            eprintln!("See the crate README for installation instructions.");
            eprintln!("Remember to clone the repo recursively if building from source.");
            bail!("Aborting compilation because bundled source directory is empty.");
        }

        let build_dir = out_dir();
        let install_dir = out_dir();

        let abseil_build_dir = build_dir.join(BUNDLED_SOURCE_PATH_ABSEIL);
        std::fs::create_dir_all(&abseil_build_dir)?;
        cmake::Config::new(BUNDLED_SOURCE_PATH_ABSEIL)
            .out_dir(abseil_build_dir)
            .define("CMAKE_INSTALL_PREFIX", install_dir.to_str().unwrap())
            .uses_cxx11()
            .build();

        let webrtc_build_dir = build_dir.join(BUNDLED_SOURCE_PATH);
        let mut meson = Command::new("meson");
        let status = meson
            .env("LIBRARY_PATH", install_dir.join("lib"))
            .env("CPPFLAGS", format!("-I{}", install_dir.join("include").display()))
            .args(&["--prefix", install_dir.to_str().unwrap()])
            .arg("-Ddefault_library=static")
            .arg(webrtc_build_dir.to_str().unwrap())
            .arg(BUNDLED_SOURCE_PATH)
            .status()?;
        assert!(status.success(), "Command failed: {:?}", &meson);

        let mut ninja = Command::new("ninja");
        let status =
            ninja.args(&["-C", webrtc_build_dir.to_str().unwrap()]).arg("install").status()?;
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
        // From meson.build file, plus absl_graphcycles_internal.
        // TODO: Can we use the cmake trick? https://gitlab.freedesktop.org/pulseaudio/webrtc-audio-processing/-/commit/3f9907f93d3983033e176e95f5134a57900a7f6e
        println!("cargo:rustc-link-lib=static=absl_bad_optional_access");
        println!("cargo:rustc-link-lib=static=absl_base");
        println!("cargo:rustc-link-lib=static=absl_flags_config");
        println!("cargo:rustc-link-lib=static=absl_flags_internal");
        println!("cargo:rustc-link-lib=static=absl_flags_marshalling");
        println!("cargo:rustc-link-lib=static=absl_flags_parse");
        println!("cargo:rustc-link-lib=static=absl_flags_program_name");
        println!("cargo:rustc-link-lib=static=absl_flags_registry");
        println!("cargo:rustc-link-lib=static=absl_flags_usage");
        println!("cargo:rustc-link-lib=static=absl_flags_usage_internal");
        println!("cargo:rustc-link-lib=static=absl_int128");
        println!("cargo:rustc-link-lib=static=absl_graphcycles_internal");
        println!("cargo:rustc-link-lib=static=absl_malloc_internal");
        println!("cargo:rustc-link-lib=static=absl_raw_logging_internal");
        println!("cargo:rustc-link-lib=static=absl_spinlock_wait");
        println!("cargo:rustc-link-lib=static=absl_stacktrace");
        println!("cargo:rustc-link-lib=static=absl_str_format_internal");
        println!("cargo:rustc-link-lib=static=absl_strings");
        println!("cargo:rustc-link-lib=static=absl_symbolize");
        println!("cargo:rustc-link-lib=static=absl_synchronization");
        println!("cargo:rustc-link-lib=static=absl_throw_delegate");
        println!("cargo:rustc-link-lib=static=absl_time");
        println!("cargo:rustc-link-lib=static=absl_time_zone");

        println!("cargo:rustc-link-lib=static=webrtc-audio-processing-1");
    } else {
        println!("cargo:rustc-link-lib=dylib=webrtc-audio-processing-1");
    }

    if cfg!(target_os = "macos") {
        println!("cargo:rustc-link-lib=dylib=c++");
        println!("cargo:rustc-link-lib=framework=CoreFoundation");
    } else {
        println!("cargo:rustc-link-lib=dylib=stdc++");
    }

    cc::Build::new()
        .cpp(true)
        .file("src/wrapper.cpp")
        .flag("-std=c++11")
        .flag("-Wno-unused-parameter")
        .includes(&include_dirs)
        .out_dir(&out_dir())
        .compile("webrtc_audio_processing_wrapper");

    println!("cargo:rustc-link-lib=static=webrtc_audio_processing_wrapper");

    let binding_file = out_dir().join("bindings.rs");
    let mut builder = bindgen::Builder::default()
        .header("src/wrapper.hpp")
        .clang_args(&["-x", "c++", "-std=c++11", "-fparse-all-comments"])
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
