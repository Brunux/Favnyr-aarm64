//! The version the binary reports.
//!
//! It is the repository's committed `VERSION` file, compiled in at build time:
//! an image that says a version its `VERSION` never had is exactly the drift
//! `/etc/debian-aarch64-revision` exists to prevent, and `check-system.sh`
//! compares the two. `cargo`'s own package version is a placeholder on purpose,
//! so bumping the release does not mean editing four manifests.

/// The committed file, with its trailing newline.
pub const VERSION_FILE: &str = include_str!("../../../../VERSION");

/// The version without the newline, ready to print.
pub fn version() -> &'static str {
    VERSION_FILE.trim_end_matches(['\n', '\r'])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_matches_file() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../VERSION");
        let on_disk = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
        assert_eq!(version(), on_disk.trim());
    }

    #[test]
    fn version_is_a_plain_semver_without_whitespace() {
        assert_eq!(version().split('.').count(), 3, "VERSION is x.y.z");
        assert!(!version().contains(char::is_whitespace));
        assert!(!version().is_empty());
    }
}
