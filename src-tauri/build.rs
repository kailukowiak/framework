fn main() {
    // The Linux packages install the XGBoost runtime in /usr/lib/framework,
    // beside the /usr/bin executable that links it (see
    // tauri.linux-release.conf.json), so the executable looks there first.
    // The AppImage bundler rewrites this to its own layout, and an unbundled
    // dev build finds the staged copy through the LD_LIBRARY_PATH that
    // scripts/tauri-native.mjs sets.
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("linux") {
        println!("cargo:rustc-link-arg-bins=-Wl,-rpath,$ORIGIN/../lib/framework");
    }
    tauri_build::build()
}
