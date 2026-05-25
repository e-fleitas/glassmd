// src-tauri/src/commands.rs
//
// Tauri commands — these are the functions the frontend calls with `invoke()`.
//
// 📚 LEARNING NOTE — The Tauri command model:
//   Think of Tauri commands as a "typed REST API" between JS and Rust.
//   In JS you write:
//     const result = await invoke("convert_file", { filePath: "/path/to/file.pdf" });
//   Tauri routes this to the #[tauri::command] function with the matching name,
//   deserializes the argument, runs the function, then serializes and returns
//   the result. All of this is type-safe: if the types don't match, you get a
//   compile error, not a runtime crash.
//
//   Error handling:
//   Commands return `Result<T, E>` where E must implement `serde::Serialize`.
//   On `Err(e)`, Tauri sends `{ error: e }` to the frontend. On `Ok(v)`,
//   it sends `v` directly. The frontend sees this as a rejected or resolved Promise.

use crate::security;
use crate::sidecar;

/// The successful result of a file conversion.
#[derive(serde::Serialize)]
pub struct ConversionResult {
    /// The converted Markdown text.
    pub markdown: String,
    /// Document title if extractable (e.g., from PDF metadata).
    pub title: Option<String>,
    /// Wall-clock time the conversion took, in milliseconds.
    pub duration_ms: u64,
}

/// A structured error that the frontend can pattern-match on.
#[derive(Debug, serde::Serialize)]
pub struct CommandError {
    /// Human-readable message for display.
    pub message: String,
    /// Machine-readable type for frontend logic.
    /// e.g. "UNSUPPORTED_EXTENSION", "CONVERSION_FAILED", "PATH_INVALID"
    pub error_type: String,
}

impl From<security::SecurityError> for CommandError {
    fn from(e: security::SecurityError) -> Self {
        let error_type = match &e {
            security::SecurityError::UnsupportedExtension(_) => "UNSUPPORTED_EXTENSION",
            security::SecurityError::IsADirectory => "IS_A_DIRECTORY",
            security::SecurityError::DoesNotExist => "FILE_NOT_FOUND",
            _ => "PATH_INVALID",
        };
        Self {
            message: e.to_string(),
            error_type: error_type.to_string(),
        }
    }
}

impl From<sidecar::SidecarError> for CommandError {
    fn from(e: sidecar::SidecarError) -> Self {
        Self {
            message: e.to_string(),
            error_type: "CONVERSION_FAILED".to_string(),
        }
    }
}

/// Converts a single file to Markdown.
///
/// Called from the frontend as:
/// ```typescript
/// const result = await invoke<ConversionResult>("convert_file", {
///   filePath: "/absolute/path/to/document.pdf"
/// });
/// ```
///
/// # Arguments
/// * `file_path` — Raw file path string from the frontend. ALWAYS validated
///                 by `security::validate_file_path` before any IO.
///
/// # Returns
/// `Ok(ConversionResult)` on success, `Err(CommandError)` on failure.
#[tauri::command]
pub async fn convert_file(
    app: tauri::AppHandle,
    file_path: String,
) -> Result<ConversionResult, CommandError> {
    let start = std::time::Instant::now();

    // ── Security Gate ────────────────────────────────────────────────────────
    // This is the ONLY place file paths enter the trusted zone.
    // validate_file_path() canonicalizes the path, checks existence, and
    // validates the extension. After this call, `canonical_path` is safe.
    let canonical_path = security::validate_file_path(&file_path)?;

    // ── Conversion ───────────────────────────────────────────────────────────
    let response = sidecar::run_conversion(&app, canonical_path).await?;

    let duration_ms = start.elapsed().as_millis() as u64;

    Ok(ConversionResult {
        markdown: response.markdown.unwrap_or_default(),
        title: response.title,
        duration_ms,
    })
}

/// Returns the list of supported file extensions.
/// Used by the frontend to show in the UI and validate drag-and-drop.
///
/// ```typescript
/// const extensions = await invoke<string[]>("get_supported_extensions");
/// // ["pdf", "docx", "pptx", ...]
/// ```
#[tauri::command]
pub fn get_supported_extensions() -> Vec<String> {
    security::ALLOWED_EXTENSIONS
        .iter()
        .map(|s| s.to_string())
        .collect()
}
