use failure::Error;
use regex::Regex;
use std::{
    fs::File,
    io::{Read, Write},
    path::{Path, PathBuf},
};

fn out_dir() -> PathBuf {
    std::env::var("OUT_DIR").expect("OUT_DIR environment var not set.").into()
}

#[cfg(not(feature = "bundled"))]
mod webrtc {
    use super::*;
    use failure::bail;

    const LIB_NAME: &str = "webrtc-audio-processing";

    pub(super) fn get_build_paths() -> Result<(PathBuf, PathBuf), Error> {
        let (pkgconfig_include_path, pkgconfig_lib_path) = find_pkgconfig_paths()?;

        let include_path = std::env::var("WEBRTC_AUDIO_PROCESSING_INCLUDE")
            .ok()
            .map(|x| x.into())
            .or(pkgconfig_include_path);
        let lib_path = std::env::var("WEBRTC_AUDIO_PROCESSING_LIB")
            .ok()
            .map(|x| x.into())
            .or(pkgconfig_lib_path);

        println!("{:?}, {:?}", include_path, lib_path);

        match (include_path, lib_path) {
            (Some(include_path), Some(lib_path)) => Ok((include_path, lib_path)),
            _ => {
                eprintln!("Couldn't find either header or lib files for {}.", LIB_NAME);
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
            .probe(LIB_NAME)
            .and_then(|mut lib| Ok((lib.include_paths.pop(), lib.link_paths.pop())))?)
    }
}

#[cfg(feature = "bundled")]
mod webrtc {
    use super::*;
    const BUNDLED_SOURCE_PATH: &str = "./webrtc-audio-processing";

    pub(super) fn get_build_paths() -> Result<(PathBuf, PathBuf), Error> {
        let include_path = out_dir().join(BUNDLED_SOURCE_PATH);
        let lib_path = out_dir().join("lib");
        Ok((include_path, lib_path))
    }

    fn copy_source_to_out_dir() -> Result<PathBuf, Error> {
        use fs_extra::dir::CopyOptions;

        let out_dir = out_dir();
        let mut options = CopyOptions::new();
        options.overwrite = true;

        fs_extra::dir::copy(BUNDLED_SOURCE_PATH, &out_dir, &options)?;

        Ok(out_dir.join(BUNDLED_SOURCE_PATH))
    }

    pub(super) fn build_if_necessary() -> Result<(), Error> {
        let build_dir = copy_source_to_out_dir()?;
        if build_dir.read_dir()?.next().is_none() {
            eprintln!("The webrtc-audio-processing build directory is empty");
            eprintln!("See the crate README for installation instructions");
            eprintln!("Remember to clone the repo recursively if building from source.");
        }

        if cfg!(target_os = "macos") {
            run_command(&build_dir, "glibtoolize", None)?;
        } else {
            run_command(&build_dir, "libtoolize", None)?;
        }

        run_command(&build_dir, "aclocal", None)?;
        run_command(&build_dir, "automake", Some(&["--add-missing", "--copy"]))?;
        run_command(&build_dir, "autoconf", None)?;

        autotools::Config::new(build_dir)
            .cflag("-fPIC")
            .cxxflag("-fPIC")
            .disable_shared()
            .enable_static()
            .build();

        Ok(())
    }

    fn run_command<P: AsRef<Path>>(
        curr_dir: P,
        cmd: &str,
        args_opt: Option<&[&str]>,
    ) -> Result<(), Error> {
        let mut command = std::process::Command::new(cmd);

        command.current_dir(curr_dir);

        if let Some(args) = args_opt {
            command.args(args);
        }

        let _output = command.output().map_err(|e| {
            failure::format_err!("Error running command '{}' with args '{:?}' - {:?}", cmd, args_opt, e)
        })?;

        Ok(())
    }
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

fn main() -> Result<(), Error> {
    webrtc::build_if_necessary()?;
    let (webrtc_include, webrtc_lib) = webrtc::get_build_paths()?;

    cc::Build::new()
        .cpp(true)
        .file("src/wrapper.cpp")
        .include(&webrtc_include)
        .flag("-Wno-unused-parameter")
        .flag("-std=c++11")
        .out_dir(&out_dir())
        .compile("webrtc_audio_processing_wrapper");

    println!("cargo:rustc-link-search=native={}", webrtc_lib.display());
    println!("cargo:rustc-link-lib=static=webrtc_audio_processing_wrapper");

    if cfg!(feature = "bundled") {
        println!("cargo:rustc-link-lib=static=webrtc_audio_processing");
    } else {
        println!("cargo:rustc-link-lib=dylib=webrtc_audio_processing");
    }

    if cfg!(target_os = "macos") {
        println!("cargo:rustc-link-lib=dylib=c++");
    } else {
        println!("cargo:rustc-link-lib=dylib=stdc++");
    }

    let binding_file = out_dir().join("bindings.rs");
    bindgen::Builder::default()
        .header("src/wrapper.hpp")
        .generate_comments(true)
        .rustified_enum(".*")
        .derive_debug(true)
        .derive_default(true)
        .derive_partialeq(true)
        .clang_arg(&format!("-I{}", &webrtc_include.display()))
        .disable_name_namespacing()
        .generate()
        .expect("Unable to generate bindings")
        .write_to_file(&binding_file)
        .expect("Couldn't write bindings!");

    if cfg!(feature = "derive_serde") {
        derive_serde(&binding_file).expect("Failed to modify derive macros");
    }

    Ok(())
}
