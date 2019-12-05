use autotools;
#[cfg(not(feature = "bundled"))]
use pkg_config;
use failure::Error;
use regex::Regex;
use std::{
    fs::File,
    io::{Read, Write},
    path::Path,
    process::Command,
};


#[cfg(not(feature = "bundled"))]
const LIB_NAME: &str = "webrtc-audio-processing";
const BUNDLED_SOURCE_PATH: &str = "./webrtc-audio-processing";

#[cfg(not(feature = "bundled"))]
fn find_header_include_path() -> Option<String> {
    pkg_config::Config::new()
        .print_system_libs(false)
        .probe(LIB_NAME)
        .ok()
        .and_then(|mut lib| {
            lib.include_paths.pop()
        }).map(|header| header.to_string_lossy().into())
}

#[cfg(not(feature = "bundled"))]
fn get_header_include_path_from_env() -> Option<String> {
    std::env::var("WEBRTC_AUDIO_PROCESSING_INCLUDE").ok()
}

#[cfg(not(feature = "bundled"))]
fn get_header_include_path() -> String {
    let header = get_header_include_path_from_env()
        .or_else(find_header_include_path);

    match header {
        Some(header_path) => header_path,
        None => {
            eprintln!("Couldn't find header files for {}.", LIB_NAME);
            eprintln!("See the crate README for installation instructions, or use the 'bundled' feature to statically compile.");
            panic!("Aborting compilation due to linker failure.");
        }
    }
}

#[cfg(feature = "bundled")]
fn get_header_include_path() -> String {
    BUNDLED_SOURCE_PATH.to_string()
}

// TODO: Consider fixing this with the upstream.
// https://github.com/rust-lang/rust-bindgen/issues/1089
// https://github.com/rust-lang/rust-bindgen/issues/1301
fn derive_serde(binding_file: &Path) -> Result<(), Error> {
    let mut contents = String::new();
    File::open(binding_file)?.read_to_string(&mut contents)?;

    let new_contents = format!(
        "use serde::{{Serialize, Deserialize}};\n{}",
        Regex::new(r"#\s*\[\s*derive\s*\((?P<d>[^)]+)\)\s*\]\s*pub\s*(?P<s>struct|enum)")?
            .replace_all(&contents, "#[derive($d, Serialize, Deserialize)] pub $s")
    );

    File::create(&binding_file)?.write_all(new_contents.as_bytes())?;

    Ok(())
}

fn configure_webrtc_audio() -> Result<(), Error> {
    if cfg!(target_os = "macos") {
        run_command(BUNDLED_SOURCE_PATH, "glibtoolize", None)?;
    } else {
        run_command(BUNDLED_SOURCE_PATH, "libtoolize", None)?;
    }

    run_command(BUNDLED_SOURCE_PATH, "aclocal", None)?;
    run_command(BUNDLED_SOURCE_PATH, "automake", Some(&["--add-missing", "--copy"]))?;
    run_command(BUNDLED_SOURCE_PATH, "autoconf", None)?;

    Ok(())
}

fn run_command(curr_dir: &str, cmd: &str, args_opt: Option<&[&str]>) -> Result<(), Error> {
    let mut command = Command::new(cmd);

    command.current_dir(curr_dir);

    if let Some(args) = args_opt {
        command.args(args);
    }

    let _output = command.output()?;

    Ok(())
}

fn main() {
    if let Err(err) = configure_webrtc_audio() {
        eprintln!("Unable to configure webrtc-audio-processing: {:?}", err);
    }

    let mut config = autotools::Config::new("webrtc-audio-processing");
    if cfg!(feature = "bundled") {
            config.disable_shared()
            .enable_static();
    } else {
        config.enable_shared()
            .disable_static();
    };

    let out_path = config.build();

    cc::Build::new()
        .cpp(true)
        .file("src/wrapper.cpp")
        .include(get_header_include_path())
        .flag("-Wno-unused-parameter")
        .flag("-std=c++11")
        .out_dir(&out_path)
        .compile("webrtc_audio_processing_wrapper");

    println!("cargo:rustc-link-lib=static=webrtc_audio_processing_wrapper");
    println!("cargo:rustc-link-lib=dylib=webrtc_audio_processing");

    if cfg!(target_os = "macos") {
        println!("cargo:rustc-link-lib=dylib=c++");
    } else {
        println!("cargo:rustc-link-lib=dylib=stdc++");
    }

    println!("cargo:rustc-link-search=native={}", out_path.join("lib").display());

    let binding_file = out_path.join("bindings.rs");
    bindgen::Builder::default()
        .header("src/wrapper.hpp")
        .generate_comments(true)
        .rustified_enum(".*")
        .derive_debug(true)
        .derive_default(true)
        .derive_partialeq(true)
        .disable_name_namespacing()
        .generate()
        .expect("Unable to generate bindings")
        .write_to_file(&binding_file)
        .expect("Couldn't write bindings!");

    if cfg!(feature = "derive_serde") {
        derive_serde(&binding_file).expect("Failed to modify derive macros");
    }
}
