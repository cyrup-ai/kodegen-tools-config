// ============================================================================
// ENVIRONMENT VARIABLE LOADING
// ============================================================================

/// Load allowed directories from `KODEGEN_ALLOWED_DIRS` environment variable
/// Format: Colon-separated on Unix/macOS, semicolon-separated on Windows
pub(crate) fn load_allowed_dirs_from_env() -> Vec<String> {
    let separator = if cfg!(windows) { ';' } else { ':' };

    std::env::var("KODEGEN_ALLOWED_DIRS")
        .ok()
        .map(|dirs| {
            dirs.split(separator)
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect()
        })
        .unwrap_or_default()
}

/// Load denied directories from `KODEGEN_DENIED_DIRS` environment variable
/// Format: Colon-separated on Unix/macOS, semicolon-separated on Windows
pub(crate) fn load_denied_dirs_from_env() -> Vec<String> {
    let separator = if cfg!(windows) { ';' } else { ':' };

    std::env::var("KODEGEN_DENIED_DIRS")
        .ok()
        .map(|dirs| {
            dirs.split(separator)
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect()
        })
        .unwrap_or_default()
}
