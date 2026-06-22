//! Adds the embedded-test linker to test binaries.

fn main() {
    println!("cargo:rustc-link-arg-tests=-Tembedded-test.x");
}
