extern crate cmake;
extern crate cc;
extern crate pkg_config;
extern crate bindgen;

use std::env;
use std::ffi::OsString;
use std::fs::{self, File};
use std::io::prelude::*;
use std::path::{Path, PathBuf};
use std::process::Command;

fn main() {
    build_librabbitmq();

    let bindings = bindgen::Builder::default()
        // The input header we would like to generate
        // bindings for.
        .header("wrapper.h")
        // Finish the builder and generate the bindings.
        .generate()
        // Unwrap the Result and panic on failure.
        .expect("Unable to generate bindings");

    // Write the bindings to the $OUT_DIR/bindings.rs file.
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");
}

fn build_librabbitmq() {
    let has_pkgconfig = Command::new("pkg-config").output().is_ok();

    if let Ok(lib) = pkg_config::find_library("librabbitmq") {
        for path in &lib.include_paths {
            println!("cargo:include={}", path.display());
        }
        return
    }

    if !Path::new("librabbitmq/.git").exists() {
        let _ = Command::new("git").args(&["submodule", "update", "--init"])
                                   .status();
    }

    let mut cfg = cmake::Config::new("librabbitmq");


    let dst = cfg.define("BUILD_SHARED_LIBS", "OFF")
                 .define("BUILD_STATIC_LIBS", "ON")
                 .define("BUILD_EXAMPLES", "OFF")
                 .build();

    println!("cargo:rustc-link-lib=static=librabbitmq");
    println!("cargo:rustc-link-search=native={}/lib", dst.display());
}

fn register_dep(dep: &str) {
    if let Some(s) = env::var_os(&format!("DEP_{}_ROOT", dep)) {
        prepend("PKG_CONFIG_PATH", Path::new(&s).join("lib/pkgconfig"));
        return
    }
    if let Some(s) = env::var_os(&format!("DEP_{}_INCLUDE", dep)) {
        let root = Path::new(&s).parent().unwrap();
        env::set_var(&format!("DEP_{}_ROOT", dep), root);
        let path = root.join("lib/pkgconfig");
        if path.exists() {
            prepend("PKG_CONFIG_PATH", path);
            return
        }
    }
}

fn prepend(var: &str, val: PathBuf) {
    let prefix = env::var(var).unwrap_or(String::new());
    let mut v = vec![val];
    v.extend(env::split_paths(&prefix));
    env::set_var(var, &env::join_paths(v).unwrap());
}


