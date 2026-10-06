//! The desktop player: everything lives in the library, see `lib.rs`.

// release builds on Windows open as a window, without a console behind it
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    macguitar::run_desktop();
}
