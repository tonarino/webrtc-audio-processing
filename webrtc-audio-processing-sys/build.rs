use failure::Error;
use regex::Regex;
use std::{
    env,
    fs::File,
    io::{Read, Write},
    path::{Path, PathBuf},
};

// TODO: Consider fixing this with the upstream.
// https://github.com/rust-lang/rust-bindgen/issues/1301
fn add_derives(binding_file: &Path) -> Result<(), Error> {
    let mut contents = String::new();
    File::open(binding_file)?.read_to_string(&mut contents)?;

    // Add PartialEq to structs.
    // Used for checking partial equality of `Config` struct.
    let contents = Regex::new(r"#\s*\[\s*derive\s*\((?P<d>[^)]+)\)\s*\]\s*pub struct")?
        .replace_all(&contents, "#[derive($d, PartialEq)]\n pub struct");

    #[cfg(feature = "derive_serde")] {
        // Add Serialize and Deserialize derive to enums and structs.
        let contents = Regex::new(r"#\s*\[\s*derive\s*\((?P<d>[^)]+)\)\s*\]\s*pub struct|enum")?
            .replace_all(&contents, "#[derive($d, Serialize, Deserialize)]\n pub struct");
    }

    let new_binding_contents = format!("use serde::{{Serialize, Deserialize}};\n{}", contents);
    File::create(&binding_file)?.write_all(new_binding_contents.as_bytes())?;

    Ok(())
}

fn main() {
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());

    cc::Build::new()
        .cpp(true)
        .file("src/wrapper.cpp")
        .include("/usr/include/webrtc_audio_processing")
        .include("/usr/local/include/webrtc_audio_processing")
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
