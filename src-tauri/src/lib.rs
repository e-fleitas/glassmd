// src-tauri/src/lib.rs
//
// Library root — declares modules and wires up the Tauri application.
//
// 📚 LEARNING NOTE — Why `lib.rs` + `main.rs`?
//   Rust projects can be either a binary crate (main.rs) OR a library crate
//   (lib.rs). Tauri requires BOTH. `lib.rs` lets us unit-test our commands
//   without starting the full Tauri runtime. `main.rs` is just the OS entry
//   point that calls into the lib.

mod commands;  // Tauri command handlers (the "API" the frontend calls)
mod security;  // Path validation — our security boundary
mod sidecar;   // Python process management

use tauri::Manager;

/// Application state shared across all Tauri commands.
/// Stored in Tauri's managed state — think of it as a thread-safe global.
pub struct AppState {
    // Reserved for future use: conversion queue, settings, etc.
}

/// Runs the Tauri application.
/// Called by `main.rs` on startup.
pub fn run() {
    tauri::Builder::default()
        // ── Plugins ──────────────────────────────────────────────────────────
        // Each plugin must be registered here AND declared in tauri.conf.json.
        // The dialog plugin provides native OS file pickers (secure, no webview).
        .plugin(tauri_plugin_dialog::init())
        // The shell plugin is needed to spawn our Python sidecar process.
        .plugin(tauri_plugin_shell::init())
        // ── Managed State ────────────────────────────────────────────────────
        .manage(AppState {})
        // ── Command Registration ──────────────────────────────────────────────
        // Every function the frontend calls with `invoke("command_name")` must
        // be listed here. Rust will give a runtime panic if you forget one.
        .invoke_handler(tauri::generate_handler![
            commands::convert_file,
            commands::get_supported_extensions,
        ])
        // ── Window Setup ─────────────────────────────────────────────────────
        .setup(|app| {
            // In debug builds, open DevTools automatically.
            #[cfg(debug_assertions)]
            {
                let window = app.get_webview_window("main").unwrap();
                window.open_devtools();
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
