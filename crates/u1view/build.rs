fn main() {
    // A static SDL2 (`--features u1/bundled`, the Windows CI build) calls
    // the registry API; the sdl2 crate does not link its library itself.
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        println!("cargo:rustc-link-lib=advapi32");
    }
}
