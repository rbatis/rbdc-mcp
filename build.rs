fn main() {
    // rbdc_sqlite (via libsqlite3-sys) and rbdc_turso (via libsql-ffi) both
    // statically bundle their own copy of sqlite3.c.  When both features are
    // enabled together the linker sees duplicate symbols.
    //
    // We inject the "allow multiple definition" flag via build.rs rather than
    // .cargo/config.toml because cargo install --git may not carry the
    // project's .cargo/ directory into the build directory.

    let has_sqlite = std::env::var("CARGO_FEATURE_SQLITE").is_ok();
    let has_turso = std::env::var("CARGO_FEATURE_TURSO").is_ok();

    if has_sqlite && has_turso {
        let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
        match target_os.as_str() {
            "windows" => {
                // MSVC linker
                println!("cargo:rustc-link-arg=/FORCE:MULTIPLE");
            }
            "linux" => {
                // GNU/LLD linker
                println!("cargo:rustc-link-arg=-Wl,--allow-multiple-definition");
            }
            _ => {}
        }
    }
}
