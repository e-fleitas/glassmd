// src-tauri/src/sidecar.rs
//
// Python sidecar process management.
//
// 📚 LEARNING NOTE — What is a "sidecar"?
//   In Tauri, a "sidecar" is an external binary bundled with your app. Tauri
//   takes care of locating it at runtime regardless of OS (Windows adds .exe,
//   paths differ per platform, etc.). We use this to ship a compiled Python
//   executable so the user doesn't need Python installed.
//
//   The sidecar communication model:
//     Rust spawns the Python process → writes JSON to its stdin →
//     reads newline-delimited JSON from its stdout → parses and returns result.
//
//   This is called "stdio IPC" (Inter-Process Communication) and is the
//   simplest, most portable, most debuggable approach.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tauri_plugin_shell::ShellExt;

/// The request we send to the Python sidecar over stdin.
/// Matches the schema expected by `markitdown_sidecar.py`.
#[derive(Serialize)]
pub struct SidecarRequest {
    pub file_path: String,
    pub options: SidecarOptions,
}

#[derive(Serialize)]
pub struct SidecarOptions {
    pub include_metadata: bool,
}

/// The response we receive from the Python sidecar over stdout.
#[derive(Deserialize)]
pub struct SidecarResponse {
    pub status: String,           // "success" | "error"
    pub markdown: Option<String>, // Present on success
    pub title: Option<String>,    // Document title if extractable
    pub error: Option<String>,    // Present on error
    pub error_type: Option<String>, // e.g. "UnsupportedFormat", "PermissionError"
}

/// Errors specific to sidecar communication.
#[derive(Debug, thiserror::Error)]
pub enum SidecarError {
    #[error("Failed to spawn sidecar process: {0}")]
    SpawnFailed(String),

    #[error("Sidecar returned empty output")]
    EmptyOutput,

    #[error("Failed to parse sidecar response: {0}")]
    ParseFailed(String),

    #[error("Sidecar reported conversion error: {0}")]
    ConversionFailed(String),

    #[error("Tauri shell error: {0}")]
    Shell(String),
}

impl serde::Serialize for SidecarError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

/// Calls the Python sidecar to convert a file to Markdown.
///
/// Takes a validated, canonical `PathBuf` (from `security::validate_file_path`)
/// and returns the Markdown string or a `SidecarError`.
///
/// 📚 LEARNING NOTE — `async fn` and `await`:
///   Spawning a process and waiting for it to finish is I/O-bound: the CPU does
///   nothing while Python is running. Rust's `async`/`await` lets other tasks
///   run during that wait. Tauri commands must be `async` to avoid blocking the
///   UI thread.
pub async fn run_conversion(
    app: &tauri::AppHandle,
    validated_path: PathBuf,
) -> Result<SidecarResponse, SidecarError> {
    // Serialize our request as a single JSON line.
    // The sidecar reads one line from stdin, processes it, writes one line to
    // stdout, then exits. Simple and stateless.
    let request = SidecarRequest {
        file_path: validated_path.to_string_lossy().to_string(),
        options: SidecarOptions {
            include_metadata: false,
        },
    };

    let request_json = serde_json::to_string(&request)
        .map_err(|e| SidecarError::ParseFailed(e.to_string()))?;

    // Ask Tauri to find and spawn our sidecar binary.
    // The name "markitdown-sidecar" maps to:
    //   - "binaries/markitdown-sidecar-x86_64-pc-windows-msvc.exe" on Windows
    //   - "binaries/markitdown-sidecar-x86_64-unknown-linux-gnu" on Linux
    // Tauri auto-detects the right triple — that's the magic of sidecars.
    let sidecar_cmd = app
        .shell()
        .sidecar("markitdown-sidecar")
        .map_err(|e| SidecarError::SpawnFailed(e.to_string()))?
        .arg(request_json); // Pass request as a CLI argument

    // `output()` runs the process to completion and captures stdout/stderr.
    // This is blocking at the sidecar level but async at the Tauri level.
    let output = sidecar_cmd
        .output()
        .await
        .map_err(|e| SidecarError::Shell(e.to_string()))?;

    // Parse the stdout as UTF-8 JSON.
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stdout = stdout.trim();

    if stdout.is_empty() {
        // If stdout is empty, check stderr for clues.
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(SidecarError::EmptyOutput);
    }

    let response: SidecarResponse = serde_json::from_str(stdout)
        .map_err(|e| SidecarError::ParseFailed(format!("{e}: raw='{stdout}'")))?;

    if response.status == "error" {
        let msg = response.error.clone().unwrap_or_else(|| "Unknown error".into());
        return Err(SidecarError::ConversionFailed(msg));
    }

    Ok(response)
}
