use anyhow::{Context, Result};
use std::{env, path::PathBuf, process::Command};

const DEPLOYMENT_TARGET_VAR: &str = "MACOSX_DEPLOYMENT_TARGET";

/// Symbol prefix for the webrtc-audio-processing library to allow multiple versions to coexist.
const SYMBOL_PREFIX: &str = "v2_";

fn out_dir() -> PathBuf {
    std::env::var("OUT_DIR").expect("OUT_DIR environment var not set.").into()
}

fn src_dir() -> PathBuf {
    std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR environment var not set.").into()
}

/// Prefix all symbols in a static library using objcopy.
/// This modifies the library in-place.
/// Note: This only works on Linux. macOS's llvm-objcopy doesn't support --prefix-symbols for Mach-O.
fn prefix_symbols_in_archive(archive_path: &std::path::Path, prefix: &str) -> Result<()> {
    if cfg!(target_os = "macos") {
        // llvm-objcopy doesn't support --prefix-symbols for Mach-O format
        eprintln!(
            "Warning: Symbol prefixing is not supported on macOS. \
            Linking multiple versions of this crate may cause symbol conflicts."
        );
        return Ok(());
    }

    eprintln!("Prefixing symbols in {} with '{}'", archive_path.display(), prefix);

    let temp_path = archive_path.with_extension("prefixed.a");

    let status = Command::new("objcopy")
        .arg(format!("--prefix-symbols={}", prefix))
        .arg(archive_path)
        .arg(&temp_path)
        .status()
        .context("Failed to execute objcopy")?;

    if !status.success() {
        anyhow::bail!("objcopy failed with status: {}", status);
    }

    std::fs::rename(&temp_path, archive_path).with_context(|| {
        format!("Failed to rename {} to {}", temp_path.display(), archive_path.display())
    })?;

    Ok(())
}

/// Bindgen callback to prefix link names.
#[derive(Debug)]
struct PrefixRenamer {
    prefix: String,
}

impl bindgen::callbacks::ParseCallbacks for PrefixRenamer {
    fn generated_link_name_override(
        &self,
        item_info: bindgen::callbacks::ItemInfo<'_>,
    ) -> Option<String> {
        Some(format!("{}{}", self.prefix, item_info.name))
    }
}

#[cfg(not(feature = "bundled"))]
mod webrtc {
    use super::*;
    use anyhow::{bail, Result};

    const LIB_NAME: &str = "webrtc-audio-processing-2";
    const LIB_MIN_VERSION: &str = "2.1";

    pub(super) fn get_build_paths() -> Result<(Vec<PathBuf>, Vec<PathBuf>)> {
        let (pkgconfig_include_path, pkgconfig_lib_path) = find_pkgconfig_paths()?;

        let include_path = std::env::var("WEBRTC_AUDIO_PROCESSING_INCLUDE")
            .ok()
            .map(PathBuf::from)
            .or(pkgconfig_include_path);
        let lib_path = std::env::var("WEBRTC_AUDIO_PROCESSING_LIB")
            .ok()
            .map(PathBuf::from)
            .or(pkgconfig_lib_path);

        if include_path.is_none() || lib_path.is_none() {
            bail!(
                "Couldn't find {}. Please install it or set WEBRTC_AUDIO_PROCESSING_INCLUDE and WEBRTC_AUDIO_PROCESSING_LIB environment variables.",
                LIB_NAME
            );
        }

        Ok((vec![include_path.unwrap()], vec![lib_path.unwrap()]))
    }

    pub(super) fn build_if_necessary() -> Result<()> {
        Ok(())
    }

    fn find_pkgconfig_paths() -> Result<(Option<PathBuf>, Option<PathBuf>)> {
        let lib = match pkg_config::Config::new()
            .atleast_version(LIB_MIN_VERSION)
            .statik(false)
            .probe(LIB_NAME)
        {
            Ok(lib) => lib,
            Err(e) => {
                eprintln!("Couldn't find {LIB_NAME} with pkg-config:");
                eprintln!("{e}");
                return Ok((None, None));
            },
        };

        Ok((lib.include_paths.first().cloned(), lib.link_paths.first().cloned()))
    }

    pub(super) fn prefix_library_symbols(prefix: &str) -> Result<()> {
        // For non-bundled builds, we can't prefix symbols in the system library.
        // Users would need to build with bundled feature for multi-version support.
        eprintln!(
            "Warning: Symbol prefixing is only supported with the 'bundled' feature. \
            Without it, linking multiple versions of this crate may cause symbol conflicts."
        );

        Ok(())
    }
}

#[cfg(feature = "bundled")]
mod webrtc {
    use super::*;
    use anyhow::{bail, Context};
    use std::{path::Path, process::Command};

    const BUNDLED_SOURCE_PATH: &str = "./webrtc-audio-processing";

    pub(super) fn get_build_paths() -> Result<(Vec<PathBuf>, Vec<PathBuf>)> {
        let mut include_paths = vec![
            out_dir().join("include"),
            out_dir().join("include").join("webrtc-audio-processing-2"),
            src_dir().join("webrtc-audio-processing"),
            src_dir().join("webrtc-audio-processing").join("webrtc"),
        ];
        let mut lib_paths = vec![
            // macOS
            out_dir().join("lib"),
            // linux
            out_dir().join("lib").join("x86_64-linux-gnu"),
        ];

        // Notes: c8896801 added support for 20250814, but the meson.build is still expecting
        // >=20240722 and the subproject will fetch 20240722. If the build environment has 20250814
        // installed, it should still pick it up and build successfully, though.
        if let Ok(mut lib) =
            pkg_config::Config::new().atleast_version("20240722").probe("absl_base")
        {
            // If abseil package is installed locally, meson would have linked it for
            // webrtc-audio-processing-2. Use the same library for our wrapper, too.
            include_paths.append(&mut lib.include_paths);
            lib_paths.append(&mut lib.link_paths);
        } else {
            // Otherwise use the local build fetched and built by meson.
            include_paths.push(
                src_dir()
                    .join("webrtc-audio-processing")
                    .join("subprojects")
                    .join("abseil-cpp-20240722.0"),
            );
            lib_paths.push(
                out_dir()
                    .join("webrtc-audio-processing")
                    .join("subprojects")
                    .join("abseil-cpp-20240722.0"),
            );
        }

        Ok((include_paths, lib_paths))
    }

    pub(super) fn build_if_necessary() -> Result<()> {
        if Path::new(BUNDLED_SOURCE_PATH).read_dir()?.next().is_none() {
            eprintln!("The webrtc-audio-processing source directory is empty.");
            eprintln!("See the crate README for installation instructions.");
            eprintln!("Remember to clone the repo recursively if building from source.");
            bail!("Aborting compilation because bundled source directory is empty.");
        }

        let build_dir = out_dir();
        let install_dir = out_dir();

        let webrtc_build_dir = build_dir.join(BUNDLED_SOURCE_PATH);
        eprintln!("Building webrtc-audio-processing in {}", webrtc_build_dir.display());

        let mut meson = Command::new("meson");
        meson.args(["setup", "--prefix", install_dir.to_str().unwrap()]);
        meson.arg("--reconfigure");

        if cfg!(target_os = "macos") {
            let link_args = "['-framework', 'CoreFoundation', '-framework', 'Foundation']";
            meson.arg(format!("-Dc_link_args={}", link_args));
            meson.arg(format!("-Dcpp_link_args={}", link_args));
        }

        let status = meson
            .arg("-Ddefault_library=static")
            .arg(BUNDLED_SOURCE_PATH)
            .arg(webrtc_build_dir.to_str().unwrap())
            .status()
            .context("Failed to execute meson. Do you have it installed?")?;
        assert!(status.success(), "Command failed: {:?}", &meson);

        let mut ninja = Command::new("ninja");
        let status = ninja
            .current_dir(&webrtc_build_dir)
            .status()
            .context("Failed to execute ninja. Do you have it installed?")?;
        assert!(status.success(), "Command failed: {:?}", &ninja);

        let mut install = Command::new("ninja");
        let status = install
            .current_dir(&webrtc_build_dir)
            .arg("install")
            .status()
            .context("Failed to execute ninja install")?;
        assert!(status.success(), "Command failed: {:?}", &install);

        Ok(())
    }

    /// Prefix symbols in the built webrtc-audio-processing static library.
    pub(super) fn prefix_library_symbols(prefix: &str) -> Result<()> {
        // Find the library - it could be in lib/ or lib/x86_64-linux-gnu/
        let lib_candidates = [
            // macOS
            out_dir().join("lib").join("libwebrtc-audio-processing-2.a"),
            // linux
            out_dir().join("lib").join("x86_64-linux-gnu").join("libwebrtc-audio-processing-2.a"),
        ];

        for lib_path in &lib_candidates {
            if lib_path.exists() {
                prefix_symbols_in_archive(lib_path, prefix)?;
            }
        }

        Ok(())
    }
}

fn main() -> Result<()> {
    webrtc::build_if_necessary()?;
    let (include_dirs, lib_dirs) = webrtc::get_build_paths()?;

    // Prefix symbols in the webrtc library (bundled builds only)
    webrtc::prefix_library_symbols(SYMBOL_PREFIX)?;

    for dir in &lib_dirs {
        println!("cargo:rustc-link-search=native={}", dir.display());
    }

    if cfg!(feature = "bundled") {
        println!("cargo:rustc-link-lib=static=webrtc-audio-processing-2");
        println!("cargo:rustc-link-lib=absl_strings");
    } else {
        println!("cargo:rustc-link-lib=dylib=webrtc-audio-processing-2");
    }

    if cfg!(target_os = "macos") {
        println!("cargo:rustc-link-lib=framework=CoreFoundation");
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
        cc_build.flag(format!("-mmacos-version-min={}", min_version));
    }

    cc_build
        .cpp(true)
        .file("src/wrapper.cpp")
        .includes(&include_dirs)
        .flag("-std=c++17")
        .flag("-Wno-unused-parameter")
        .flag("-Wno-deprecated-declarations")
        .flag("-Wno-nullability-completeness")
        .out_dir(out_dir())
        .compile("webrtc_audio_processing_wrapper");

    // Prefix all symbols in the wrapper library so its references to webrtc symbols
    // match the prefixed webrtc library.
    let wrapper_lib = out_dir().join("libwebrtc_audio_processing_wrapper.a");
    if wrapper_lib.exists() {
        prefix_symbols_in_archive(&wrapper_lib, SYMBOL_PREFIX)?;
    }

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

    // Only add prefix callback on platforms where objcopy symbol prefixing works
    if !cfg!(target_os = "macos") {
        builder = builder.parse_callbacks(Box::new(PrefixRenamer {
            prefix: SYMBOL_PREFIX.to_string(),
        }));
    }

    for dir in &include_dirs {
        builder = builder.clang_arg(format!("-I{}", dir.display()));
    }
    builder
        .generate()
        .expect("Unable to generate bindings")
        .write_to_file(&binding_file)
        .expect("Couldn't write bindings!");

    Ok(())
}
