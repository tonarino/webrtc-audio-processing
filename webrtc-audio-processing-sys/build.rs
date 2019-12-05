use pkg_config;
use failure::Error;
use regex::Regex;
use std::{
    env,
    fs::File,
    io::{Read, Write},
    path::{Path, PathBuf},
};

const LIB_NAME: &str = "webrtc-audio-processing";

fn find_header_include_path() -> Option<String> {
    pkg_config::Config::new()
        .print_system_libs(false)
        .probe(LIB_NAME)
        .ok()
        .and_then(|mut lib| {
            lib.include_paths.pop()
        }).map(|header| header.to_string_lossy().into())
}

fn get_header_include_path_from_env() -> Option<String> {
    env::var("DEP_WEBRTC_AUDIO_PROCESSING_INCLUDE").ok()
}

fn get_header_include_path() -> String {
    get_header_include_path_from_env()
        .or_else(find_header_include_path)
        .expect(format!("Couldn't find header files for {}, aborting.", LIB_NAME).as_str())
}

// TODO: Consider fixing this with the upstream.
// https://github.com/rust-lang/rust-bindgen/issues/1301
fn add_derives(binding_file: &Path) -> Result<(), Error> {
    let mut contents = String::new();
    File::open(binding_file)?.read_to_string(&mut contents)?;

    // Add PartialEq, Serialize and Deserialize to structs.
    let contents = Regex::new(r"#\s*\[\s*derive\s*\((?P<d>[^)]+)\)\s*\]\s*pub struct")?
        .replace_all(&contents, "#[derive($d, PartialEq, Serialize, Deserialize)]\n pub struct");
    // Add Serialize and Deserialize to enums.
    let contents = Regex::new(r"#\s*\[\s*derive\s*\((?P<d>[^)]+)\)\s*\]\s*pub enum")?
        .replace_all(&contents, "#[derive($d, Serialize, Deserialize)]\n pub enum");

    let new_binding_contents = format!("use serde::{{Serialize, Deserialize}};\n{}", contents);
    File::create(&binding_file)?.write_all(new_binding_contents.as_bytes())?;

    Ok(())
}

fn main() {
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());

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
    println!("cargo:rustc-link-lib=dylib=c++");
    println!("cargo:rustc-link-search=native={}", out_path.display());

    let binding_file = out_path.join("bindings.rs");
    bindgen::Builder::default()
        .header("src/wrapper.hpp")
        .generate_comments(true)
        .rustified_enum(".*")
        .derive_debug(true)
        .derive_default(true)
        .disable_name_namespacing()
        .generate()
        .expect("Unable to generate bindings")
        .write_to_file(&binding_file)
        .expect("Couldn't write bindings!");

    add_derives(&binding_file).expect("Failed to modify derive macros");
}
