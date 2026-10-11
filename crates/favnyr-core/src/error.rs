use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("I/O: {0}")]
    Io(#[from] std::io::Error),

    #[error("trash {}: {source}", path.display())]
    Trash {
        path: std::path::PathBuf,
        #[source]
        source: trash::Error,
    },

    #[error("{message}: {source}")]
    Operation {
        message: String,
        #[source]
        source: Box<Error>,
    },

    #[error("invalid config: {0}")]
    Config(String),

    #[error("workspace: {0}")]
    Workspace(String),

    #[error("favorites: {0}")]
    Favorites(String),

    #[error("openers: {0}")]
    Openers(String),
}

pub type Result<T> = std::result::Result<T, Error>;

impl Error {
    /// Preserves a filesystem failure category through operation context.
    pub fn io_kind(&self) -> Option<std::io::ErrorKind> {
        match self {
            Self::Io(error) => Some(error.kind()),
            Self::Operation { source, .. } => source.io_kind(),
            Self::Trash { source, .. } => trash_error_kind(source),
            _ => None,
        }
    }
}

fn trash_error_kind(error: &trash::Error) -> Option<std::io::ErrorKind> {
    match error {
        #[cfg(windows)]
        trash::Error::Os { code, .. } => windows_hresult_kind(*code),
        #[cfg(all(
            unix,
            not(target_os = "macos"),
            not(target_os = "ios"),
            not(target_os = "android")
        ))]
        trash::Error::FileSystem { source, .. } => Some(source.kind()),
        // CouldNotAccess can also mean a missing path, so it is not proof of
        // a permission denial. Unknown failures keep their category unspecified.
        _ => None,
    }
}

/// Classifies Shell HRESULTs without treating their low word as a Win32
/// code unless the HRESULT belongs to FACILITY_WIN32.
#[cfg(windows)]
pub fn windows_hresult_kind(code: i32) -> Option<std::io::ErrorKind> {
    use std::io::ErrorKind;
    let code = code as u32;
    if code & 0xffff_0000 == 0x8007_0000 {
        return Some(std::io::Error::from_raw_os_error((code & 0xffff) as i32).kind());
    }
    match code {
        // COPYENGINE_E_REQUIRES_ELEVATION, ACCESS_DENIED_SRC and ACCESS_DENIED_DEST.
        0x8027_0002 | 0x8027_0021 | 0x8027_0022 => Some(ErrorKind::PermissionDenied),
        // COPYENGINE_E_ACCESSDENIED_READONLY.
        0x8027_003f => Some(ErrorKind::ReadOnlyFilesystem),
        // COPYENGINE_E_PATH_NOT_FOUND_SRC and PATH_NOT_FOUND_DEST.
        0x8027_0023 | 0x8027_0024 => Some(ErrorKind::NotFound),
        // COPYENGINE_E_DISK_FULL and DISK_FULL_CLEAN.
        0x8027_0032 | 0x8027_0033 => Some(ErrorKind::StorageFull),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::ErrorKind;

    #[test]
    fn filesystem_categories_survive_operation_context() {
        let error = Error::Operation {
            message: "file operation failed".into(),
            source: Box::new(Error::Io(ErrorKind::PermissionDenied.into())),
        };
        assert_eq!(error.io_kind(), Some(ErrorKind::PermissionDenied));
        assert_eq!(
            Error::Workspace("unclassified failure".into()).io_kind(),
            None
        );
    }

    #[test]
    fn an_ambiguous_trash_failure_does_not_invent_permissions() {
        let error = Error::Trash {
            path: "my_file.txt".into(),
            source: trash::Error::CouldNotAccess {
                target: "my_file.txt".into(),
            },
        };
        assert_eq!(error.io_kind(), None);
    }

    #[cfg(windows)]
    #[test]
    fn trash_hresult_keeps_access_denied_and_shell_codes_distinct() {
        for code in [0x8007_0005_u32, 0x8027_0002, 0x8027_0021, 0x8027_0022] {
            let error = Error::Trash {
                path: "my_file.txt".into(),
                source: trash::Error::Os {
                    code: code as i32,
                    description: "refused".into(),
                },
            };
            assert_eq!(error.io_kind(), Some(ErrorKind::PermissionDenied));
        }
        assert_eq!(
            windows_hresult_kind(0x8027_003f_u32 as i32),
            Some(ErrorKind::ReadOnlyFilesystem)
        );
        assert_eq!(windows_hresult_kind(0x8000_0005_u32 as i32), None);
        assert_eq!(windows_hresult_kind(0x8027_0000_u32 as i32), None);
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn freedesktop_trash_keeps_the_filesystem_error_category() {
        let error = Error::Trash {
            path: "my_file.txt".into(),
            source: trash::Error::FileSystem {
                path: "my_file.txt".into(),
                source: ErrorKind::PermissionDenied.into(),
            },
        };
        assert_eq!(error.io_kind(), Some(ErrorKind::PermissionDenied));
    }
}
