extern crate cc;
extern crate cmake;

use std::path::Path;
use std::process::Command;

fn main() {
    if !Path::new("librabbitmq/.git").exists() {
        let _ = Command::new("git")
            .args(&["submodule", "update", "--init"])
            .status();
    }
    let mut cfg = cmake::Config::new("librabbitmq");

    let dst = cfg.define("BUILD_EXAMPLES", "OFF")
        .define("BUILD_SHARED_LIBS", "OFF")
        .define("BUILD_STATIC_LIBS", "ON")
        .define("BUILD_TESTS", "OFF")
        .define("BUILD_TOOLS", "ON")
        .define("BUILD_TOOLS_DOCS", "OFF")
        .define("ENABLE_SSL_SUPPORT", "ON")
        .define("ENABLE_THREAD_SAFETY", "ON")
        .define("BUILD_API_DOCS", "OFF")
        .register_dep("OPENSSL")
        .build();

    println!(
        "cargo:rustc-link-search=native={}/lib/x86_64-linux-gnu",
        dst.display()
    );
    println!("cargo:rustc-link-lib=static=rabbitmq");
}
