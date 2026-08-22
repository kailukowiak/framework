// Release builds link against the Windows GUI subsystem so launching the app
// does not also open a console window behind it. Debug builds keep the console
// so `cargo run` / `tauri dev` still show panics and test output.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    framework_desktop_lib::run();
}
