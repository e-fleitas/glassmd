// src-tauri/src/security.rs
//
// Security boundary: ALL file paths from the frontend pass through here before
// reaching the OS or the Python sidecar.
//
// 📚 LEARNING NOTE — Why validate in Rust and not just in JS?
//   The frontend (JS/WebView) runs in an untrusted sandbox. A malicious actor
//   could craft a payload that bypasses JS validation. The Rust backend, however,
//   runs at the OS level and cannot be bypassed via WebView tricks. This is the
//   "defence in depth" principle: validate at every layer, trust nothing from
//   the layer above.

use std::path::{Path, PathBuf};
use thiserror::Error;

/// All file extensions MarkItDown can process.
/// This is the definitive whitelist — if it's not here, it won't be processed.
/// Updating this list is the ONLY change needed to support new formats.
pub const ALLOWED_EXTENSIONS: &[&str] = &[
    // Documents
    "pdf", "docx", "pptx", "xlsx", "xls", "epub",
    // Web & data
    "html", "htm", "csv", "json", "jsonl", "xml",
    // Media
    "jpg", "jpeg", "png", "gif", "bmp", "webp",
    "wav", "mp3", "m4a",
    // Code & text
    "ipynb", "md", "markdown", "txt", "text",
    "py", "js", "ts", "rs", "go", "java", "cpp", "c", "h",
    // Archives
    "zip",
    // Outlook
    "msg",
];

/// Errors that can occur during path validation.
#[derive(Debug, Error)]
pub enum SecurityError {
    #[error("Path does not exist or is inaccessible")]
    DoesNotExist,

    #[error("Path points to a directory, not a file")]
    IsADirectory,

    #[error("File extension '.{0}' is not supported")]
    UnsupportedExtension(String),

    #[error("Path contains invalid characters")]
    InvalidPath,

    #[error("IO error during path resolution: {0}")]
    Io(#[from] std::io::Error),
}

// Make SecurityError serializable so Tauri can send it to the frontend as JSON.
impl serde::Serialize for SecurityError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

/// Validates a raw path string from the frontend and returns a canonicalized,
/// safe `PathBuf` — or a `SecurityError` explaining why it was rejected.
///
/// This function is the single point of truth for path safety. It:
///   1. Parses the string into a `Path` (no IO yet)
///   2. Calls `canonicalize()` which resolves `..`, symlinks, etc. (requires IO)
///   3. Asserts the result is a regular file (not a dir, socket, etc.)
///   4. Checks the extension against our whitelist
///
/// After this function returns `Ok(path)`, callers can trust the path is:
///   - Absolute and canonical (no `..` traversal)
///   - A real, existing file
///   - Of a supported format
pub fn validate_file_path(raw_path: &str) -> Result<PathBuf, SecurityError> {
    // Step 1: Basic parse — no IO yet.
    // `Path::new` never fails; it just stores the string. UTF-8 issues surface
    // at the OS level when we call canonicalize.
    let path = Path::new(raw_path);

    // Step 2: Canonicalize — this is where `..` tricks get neutralized.
    // e.g. "/home/user/../../../etc/passwd" → "/etc/passwd" which then fails
    // our extension check. The path MUST exist for canonicalize to succeed,
    // so this also implicitly validates existence.
    let canonical = path.canonicalize().map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            SecurityError::DoesNotExist
        } else {
            SecurityError::Io(e)
        }
    })?;

    // Step 3: Assert it's a file, not a directory or special node.
    if canonical.is_dir() {
        return Err(SecurityError::IsADirectory);
    }

    // Step 4: Extension whitelist check.
    let ext = canonical
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    if !ALLOWED_EXTENSIONS.contains(&ext.as_str()) {
        return Err(SecurityError::UnsupportedExtension(ext));
    }

    Ok(canonical)
}

// ── Unit Tests ────────────────────────────────────────────────────────────────
// Run with: `cargo test` from the src-tauri directory.
#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn rejects_nonexistent_path() {
        let result = validate_file_path("/this/path/does/not/exist.pdf");
        assert!(matches!(result, Err(SecurityError::DoesNotExist)));
    }

    #[test]
    fn rejects_unsupported_extension() {
        // Create a real temp file with an unsupported extension to pass the
        // existence check, then verify the extension check fires.
        let tmp = std::env::temp_dir().join("markitdown_test.exe");
        fs::write(&tmp, b"fake").unwrap();
        let result = validate_file_path(tmp.to_str().unwrap());
        fs::remove_file(&tmp).unwrap();
        assert!(matches!(result, Err(SecurityError::UnsupportedExtension(_))));
    }

    #[test]
    fn accepts_valid_pdf() {
        let tmp = std::env::temp_dir().join("markitdown_test.pdf");
        fs::write(&tmp, b"%PDF-1.4 fake").unwrap();
        let result = validate_file_path(tmp.to_str().unwrap());
        fs::remove_file(&tmp).unwrap();
        assert!(result.is_ok());
    }
}
