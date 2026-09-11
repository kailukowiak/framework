// Release builds link against the Windows GUI subsystem so launching the app
// does not also open a console window behind it. Debug builds keep the console
// so `cargo run` / `tauri dev` still show panics and test output.
//
// `dev-bundle` is a debug build that is installed and launched from Explorer
// instead, where that console has nothing to print into and no one to read it:
// it arrives as a second window whose close button kills the app mid-edit. So
// the subsystem follows how the build is *launched*, not how it was compiled.
#![cfg_attr(
    any(not(debug_assertions), feature = "dev-bundle"),
    windows_subsystem = "windows"
)]

fn main() {
    framework_desktop_lib::run();
}
