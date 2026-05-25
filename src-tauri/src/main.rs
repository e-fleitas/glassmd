// src-tauri/src/main.rs
//
// Tauri application entry point.
//
// ⚠️  Tauri requires this file to be named `main.rs` and the first call to be
//    `tauri::Builder::default()`. All heavy logic lives in `lib.rs` and the
//    sub-modules it declares. This keeps `main.rs` minimal and testable.

// Prevents an additional console window from appearing on Windows in release mode.
// This is a Tauri convention — DO NOT REMOVE.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    markitdown_gui_lib::run();
}
