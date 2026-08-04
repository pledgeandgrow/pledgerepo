//! Windows long path handling.
//!
//! Windows has a default 260-character path limit (`MAX_PATH`).
//! Paths that exceed this limit fail with cryptic I/O errors unless
//! they are prefixed with `\\?\` to opt into the extended-length path API.
//!
//! On non-Windows platforms these functions are no-ops.

use std::path::{Path, PathBuf};

/// The Windows `MAX_PATH` limit (260 characters).
const MAX_PATH: usize = 260;

/// Prefix applied to opt into Windows extended-length path support.
#[cfg(windows)]
const LONG_PATH_PREFIX: &str = r"\\?\";

/// Normalize a path for filesystem operations.
///
/// On Windows, if the absolute path length exceeds `MAX_PATH` (260) and the
/// path is not already prefixed with `\\?\`, this function returns a new
/// `PathBuf` with the `\\?\` prefix prepended.
///
/// On non-Windows platforms, this is a no-op that returns the input path.
///
/// # Example
///
/// ```no_run
/// use pledgepack_core::paths::long_path;
/// use std::path::Path;
///
/// let path = Path::new("some/very/long/path/...");
/// let safe = long_path(path);
/// std::fs::write(safe, b"hello").unwrap();
/// ```
pub fn long_path(path: &Path) -> PathBuf {
    #[cfg(windows)]
    {
        let path_str = path.to_string_lossy();
        // Already prefixed — nothing to do.
        if path_str.starts_with(LONG_PATH_PREFIX) {
            return path.to_path_buf();
        }
        // Only apply prefix for absolute paths that exceed MAX_PATH.
        // Relative paths cannot use the \\?\ prefix.
        if path.is_absolute() && path_str.len() > MAX_PATH {
            let prefixed = format!("{}{}", LONG_PATH_PREFIX, path_str);
            return PathBuf::from(prefixed);
        }
        path.to_path_buf()
    }

    #[cfg(not(windows))]
    {
        path.to_path_buf()
    }
}

/// Normalize a path for directory creation operations.
///
/// This is the same as [`long_path`] but specifically for `create_dir_all`
/// calls. On Windows, `create_dir_all` also benefits from the `\\?\` prefix
/// when any intermediate path component would exceed `MAX_PATH`.
pub fn long_dir(path: &Path) -> PathBuf {
    long_path(path)
}

/// Resolve a file path with case-insensitive fallback.
///
/// On case-sensitive filesystems (macOS APFS case-sensitive, Linux ext4),
/// `app/Page.tsx` and `app/page.tsx` are different files. Users coming from
/// macOS (case-insensitive HFS+) or Windows may accidentally use wrong casing.
///
/// This function first tries the exact path. If the file doesn't exist, it
/// walks the path components and tries to find a case-insensitive match
/// in each parent directory.
///
/// Returns the resolved path if found, or the original path if not.
///
/// # Example
///
/// ```no_run
/// use pledgepack_core::paths::resolve_case_insensitive;
/// use std::path::Path;
///
/// // User wrote "page.tsx" but file is "Page.tsx"
/// let path = Path::new("app/page.tsx");
/// let resolved = resolve_case_insensitive(path);
/// // resolved now points to app/Page.tsx
/// ```
pub fn resolve_case_insensitive(path: &Path) -> PathBuf {
    // Fast path: if the file exists as-is, no resolution needed.
    if path.exists() {
        return path.to_path_buf();
    }

    // Walk the path components and try case-insensitive matching.
    let mut resolved = PathBuf::new();
    let components: Vec<_> = path.components().collect();

    for component in components.iter() {
        let comp_str = component.as_os_str().to_string_lossy();
        let candidate = resolved.join(comp_str.as_ref());

        if candidate.exists() {
            resolved = candidate;
            continue;
        }

        // Try case-insensitive match in the parent directory.
        let parent = if resolved.as_os_str().is_empty() {
            Path::new(".")
        } else {
            &resolved
        };

        if let Ok(entries) = std::fs::read_dir(parent) {
            let comp_lower = comp_str.to_lowercase();
            let mut found = false;
            for entry in entries.flatten() {
                let entry_name = entry.file_name();
                if entry_name.to_string_lossy().to_lowercase() == comp_lower {
                    resolved = entry.path();
                    found = true;
                    break;
                }
            }
            if !found {
                // No case-insensitive match — keep the original component.
                // This preserves the original path for error messages.
                resolved = resolved.join(comp_str.as_ref());
            }
        } else {
            // Can't read directory — keep the original component.
            resolved = resolved.join(comp_str.as_ref());
        }
    }

    resolved
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_short_path_unchanged() {
        let path = Path::new("src/index.tsx");
        let result = long_path(path);
        assert_eq!(result, path);
    }

    #[test]
    fn test_already_prefixed_unchanged() {
        #[cfg(windows)]
        {
            let path = Path::new(r"\\?\C:\very\long\path");
            let result = long_path(path);
            assert_eq!(result, path);
        }
        #[cfg(not(windows))]
        {
            let path = Path::new("/usr/local/bin");
            let result = long_path(path);
            assert_eq!(result, path);
        }
    }

    #[cfg(windows)]
    #[test]
    fn test_long_absolute_path_gets_prefix() {
        // Build a path longer than MAX_PATH (260 chars).
        let mut long = String::from("C:\\");
        for _ in 0..130 {
            long.push_str("subdir\\");
        }
        long.push_str("file.tsx");
        assert!(long.len() > MAX_PATH);

        let path = Path::new(&long);
        let result = long_path(path);
        let result_str = result.to_string_lossy();
        assert!(
            result_str.starts_with(LONG_PATH_PREFIX),
            "Long path should be prefixed with \\\\?\\, got: {}",
            result_str
        );
    }

    #[cfg(windows)]
    #[test]
    fn test_long_relative_path_not_prefixed() {
        // Relative paths cannot use the \\?\ prefix.
        let mut long = String::new();
        for _ in 0..130 {
            long.push_str("subdir/");
        }
        long.push_str("file.tsx");
        assert!(long.len() > MAX_PATH);

        let path = Path::new(&long);
        let result = long_path(path);
        assert_eq!(result, path, "Relative paths should not be prefixed");
    }

    // ── Goal 92: Case-insensitive file resolution ──

    use tempfile::TempDir;

    #[test]
    fn test_case_insensitive_exact_match() {
        let tmp = TempDir::new().unwrap();
        let file_path = tmp.path().join("page.tsx");
        std::fs::write(&file_path, "test").unwrap();

        let result = resolve_case_insensitive(&file_path);
        assert_eq!(result, file_path, "Exact match should return same path");
    }

    #[test]
    fn test_case_insensitive_wrong_case() {
        let tmp = TempDir::new().unwrap();
        // Create file as Page.tsx (capital P)
        let actual_file = tmp.path().join("Page.tsx");
        std::fs::write(&actual_file, "test").unwrap();

        // Look for page.tsx (lowercase p) — should find Page.tsx
        let lookup_path = tmp.path().join("page.tsx");
        let result = resolve_case_insensitive(&lookup_path);
        assert!(
            result.exists(),
            "Case-insensitive resolution should find the file"
        );
        // On case-insensitive filesystems (Windows, macOS HFS+), the file
        // already exists at the lookup path, so the function returns it as-is.
        // On case-sensitive filesystems (Linux, macOS APFS case-sensitive),
        // the function resolves to the actual file name.
        let result_name = result.file_name().unwrap().to_string_lossy().to_lowercase();
        assert_eq!(
            result_name, "page.tsx",
            "Should resolve to a file with the same name (case-insensitive)"
        );
    }

    #[test]
    fn test_case_insensitive_nonexistent_file() {
        let tmp = TempDir::new().unwrap();
        let lookup_path = tmp.path().join("nonexistent.tsx");
        let result = resolve_case_insensitive(&lookup_path);
        // Should return the original path (which doesn't exist)
        assert_eq!(result, lookup_path, "Nonexistent file should return original path");
    }

    #[test]
    fn test_case_insensitive_nested_directory() {
        let tmp = TempDir::new().unwrap();
        // Create App/Page.tsx (capital A and P)
        let app_dir = tmp.path().join("App");
        std::fs::create_dir_all(&app_dir).unwrap();
        let actual_file = app_dir.join("Page.tsx");
        std::fs::write(&actual_file, "test").unwrap();

        // Look for app/page.tsx — should find App/Page.tsx
        let lookup_path = tmp.path().join("app").join("page.tsx");
        let result = resolve_case_insensitive(&lookup_path);
        assert!(
            result.exists(),
            "Case-insensitive resolution should find nested file"
        );
    }
}
