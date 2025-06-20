use std::fs;
use std::path::PathBuf;
use std::{env, path::Path};
use std::{
    fs::File,
    io::{BufReader, prelude::*},
};

fn lines_from_file(basepath: PathBuf, filename: impl AsRef<Path>) -> Vec<PathBuf> {
    let file = File::open(filename).expect("no such file");
    let buf = BufReader::new(file);
    buf.lines()
        .map(|l| {
            basepath
                .clone()
                .join(PathBuf::from(l.expect("Could not parse line")))
        })
        .collect()
}

fn main() {
    // If running Clippy, skip building Stim.
    if std::env::var("CLIPPY_ARGS").is_ok() {
        println!("cargo:warning=Skipping external C build in Clippy mode");
        return;
    }

    println!("cargo:rerun-if-changed=upstream");
    println!("cargo:rerun-if-changed=c_interface");
    let target_triple = std::env::var("TARGET").unwrap();
    let out_dir_path = PathBuf::from(env::var("OUT_DIR").expect("Cannot find OUT_DIR"));

    match target_triple.as_str() {
        "x86_64-unknown-linux-gnu" => x86_64_unknown_linux_gnu(out_dir_path),
        "aarch64-unknown-linux-gnu" => aarch64_unknown_linux_gnu(out_dir_path),
        "aarch64-apple-darwin" => aarch64_apple_darwin(out_dir_path),
        "x86_64-apple-darwin" => x86_64_apple_darwin(out_dir_path),
        "x86_64-pc-windows-msvc" => x86_64_pc_windows_msvc(out_dir_path),
        "x86_64-pc-windows-gnu" => x86_64_pc_windows_gnu(out_dir_path),
        _ => panic!("unknown target: {target_triple}"),
    }
}

fn x86_64_unknown_linux_gnu(out_dir: PathBuf) {
    let base_path = Path::new("upstream");
    let src_path = base_path.join("src");
    let c_interface_path = base_path.join("c_interface");
    let mut files = lines_from_file(
        base_path.to_path_buf(),
        base_path.join("file_lists").join("source_files_no_main"),
    );
    files.push(Path::new("c_interface").join("wrapper.cpp"));
    let out_path = out_dir.join("build");
    fs::create_dir_all(out_path.clone()).expect("Cannot create directory for library");
    cc::Build::new()
        .cpp(true)
        .include(src_path)
        .include(c_interface_path)
        .files(files)
        .opt_level(2)
        .debug(false)
        .warnings(false)
        .static_flag(true)
        .out_dir(out_path.clone())
        .flag("-std=c++2a")
        .compile("stim");

    println!("cargo:rustc-link-search=native={}", out_path.display());
    println!("cargo:rustc-link-lib=static=stim");
    println!("cargo:rustc-link-lib=stdc++")
}

fn aarch64_unknown_linux_gnu(out_dir: PathBuf) {
    let base_path = Path::new("upstream");
    let src_path = base_path.join("src");
    let c_interface_path = base_path.join("c_interface");
    let mut files = lines_from_file(
        base_path.to_path_buf(),
        base_path.join("file_lists").join("source_files_no_main"),
    );
    files.push(Path::new("c_interface").join("wrapper.cpp"));
    let out_path = out_dir.join("build");
    fs::create_dir_all(out_path.clone()).expect("Cannot create directory for library");
    cc::Build::new()
        .cpp(true)
        .include(src_path)
        .include(c_interface_path)
        .files(files)
        .opt_level(2)
        .debug(false)
        .warnings(false)
        .static_flag(true)
        .out_dir(out_path.clone())
        .flag("-std=c++20")
        .compile("stim");

    println!("cargo:rustc-link-search=native={}", out_path.display());
    println!("cargo:rustc-link-lib=static=stim");
    println!("cargo:rustc-link-lib=stdc++")
}

fn aarch64_apple_darwin(out_dir: PathBuf) {
    let base_path = Path::new("upstream");
    let src_path = base_path.join("src");
    let c_interface_path = base_path.join("c_interface");
    let mut files = lines_from_file(
        base_path.to_path_buf(),
        base_path.join("file_lists").join("source_files_no_main"),
    );
    files.push(Path::new("c_interface").join("wrapper.cpp"));
    let out_path = out_dir.join("build");
    fs::create_dir_all(out_path.clone()).expect("Cannot create directory for library");
    cc::Build::new()
        .cpp(true)
        .include(src_path)
        .include(c_interface_path)
        .files(files)
        .opt_level(2)
        .debug(false)
        .warnings(false)
        .static_flag(true)
        .out_dir(out_path.clone())
        .flag("-std=c++20")
        .compile("stim");

    println!("cargo:rustc-link-search=native={}", out_path.display());
    println!("cargo:rustc-link-lib=static=stim");
    println!("cargo:rustc-link-lib=c++")
}
fn x86_64_apple_darwin(out_dir: PathBuf) {
    let base_path = Path::new("upstream");
    let src_path = base_path.join("src");
    let c_interface_path = base_path.join("c_interface");
    let mut files = lines_from_file(
        base_path.to_path_buf(),
        base_path.join("file_lists").join("source_files_no_main"),
    );
    files.push(Path::new("c_interface").join("wrapper.cpp"));
    let out_path = out_dir.join("build");
    fs::create_dir_all(out_path.clone()).expect("Cannot create directory for library");
    cc::Build::new()
        .cpp(true)
        .include(src_path)
        .include(c_interface_path)
        .files(files)
        .opt_level(2)
        .debug(false)
        .warnings(false)
        .static_flag(true)
        .out_dir(out_path.clone())
        .flag("-std=c++20")
        .compile("stim");

    println!("cargo:rustc-link-search=native={}", out_path.display());
    println!("cargo:rustc-link-lib=static=stim");
    println!("cargo:rustc-link-lib=c++")
}

fn x86_64_pc_windows_msvc(out_dir: PathBuf) {
    let base_path = Path::new("upstream");
    let src_path = base_path.join("src");
    let c_interface_path = base_path.join("c_interface");
    let mut files = lines_from_file(
        base_path.to_path_buf(),
        base_path.join("file_lists").join("source_files_no_main"),
    );
    files.push(Path::new("c_interface").join("wrapper.cpp"));
    let out_path = out_dir.join("build");
    fs::create_dir_all(out_path.clone()).expect("Cannot create directory for library");
    cc::Build::new()
        .cpp(true)
        .include(src_path)
        .include(c_interface_path)
        .files(files)
        .opt_level(2)
        .debug(true)
        .warnings(false)
        .out_dir(out_path.clone())
        .flag("/std:c++20")
        .compile("stim");

    println!("cargo:rustc-link-search=native={}", out_path.display());
    println!("cargo:rustc-link-lib=static=stim");
}

fn x86_64_pc_windows_gnu(out_dir: PathBuf) {
    let base_path = Path::new("upstream");
    let src_path = base_path.join("src");
    let c_interface_path = base_path.join("c_interface");
    let mut files = lines_from_file(
        base_path.to_path_buf(),
        base_path.join("file_lists").join("source_files_no_main"),
    );
    files.push(Path::new("c_interface").join("wrapper.cpp"));
    let out_path = out_dir.join("build");
    fs::create_dir_all(out_path.clone()).expect("Cannot create directory for library");
    cc::Build::new()
        .cpp(true)
        .include(src_path)
        .include(c_interface_path)
        .files(files)
        .opt_level(2)
        .debug(false)
        .warnings(false)
        .static_flag(true)
        .out_dir(out_path.clone())
        .flag("-std=c++20")
        .compile("stim");

    println!("cargo:rustc-link-search=native={}", out_path.display());
    println!("cargo:rustc-link-lib=static=stim");
    println!("cargo:rustc-link-lib=c++")
}
