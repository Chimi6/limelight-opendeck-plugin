fn main() {
    let triple = std::env::var("TARGET").unwrap_or_default();
    println!("cargo:rustc-env=TARGET_TRIPLE={triple}");
    println!("cargo:rerun-if-changed=build.rs");
}
