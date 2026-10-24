//! Local file URI decoding shared by startup and desktop clipboard handling.

use std::ffi::OsStr;
use std::path::PathBuf;

pub fn has_file_scheme(value: &OsStr) -> bool {
    value
        .as_encoded_bytes()
        .get(..5)
        .is_some_and(|scheme| scheme.eq_ignore_ascii_case(b"file:"))
}

fn unhex(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

fn percent_decode(encoded: &[u8]) -> Option<Vec<u8>> {
    let mut decoded = Vec::with_capacity(encoded.len());
    let mut index = 0;
    while index < encoded.len() {
        if encoded[index] == b'%' {
            let high = unhex(*encoded.get(index + 1)?)?;
            let low = unhex(*encoded.get(index + 2)?)?;
            decoded.push((high << 4) | low);
            index += 3;
        } else {
            decoded.push(encoded[index]);
            index += 1;
        }
    }
    (!decoded.contains(&0)).then_some(decoded)
}

/// Returns an absolute local URI path as bytes. Remote authorities cannot be
/// interpreted as local paths, and literal query/fragment delimiters are invalid
/// in a file path URI. Encoded delimiters remain part of the filename.
pub fn decode_local_uri(uri: &[u8]) -> Option<Vec<u8>> {
    if !uri.get(..5)?.eq_ignore_ascii_case(b"file:") {
        return None;
    }
    let mut path = &uri[5..];
    if let Some(authority_and_path) = path.strip_prefix(b"//") {
        let split = authority_and_path.iter().position(|&byte| byte == b'/')?;
        let authority = &authority_and_path[..split];
        if !authority.is_empty() && !authority.eq_ignore_ascii_case(b"localhost") {
            return None;
        }
        path = &authority_and_path[split..];
    }
    if !path.starts_with(b"/") || path.iter().any(|byte| matches!(byte, b'?' | b'#')) {
        return None;
    }
    percent_decode(path)
}

pub fn to_path(uri: &OsStr) -> Option<PathBuf> {
    #[cfg(unix)]
    {
        use std::ffi::OsString;
        use std::os::unix::ffi::{OsStrExt, OsStringExt};
        decode_local_uri(uri.as_bytes()).map(|bytes| PathBuf::from(OsString::from_vec(bytes)))
    }
    #[cfg(not(unix))]
    {
        let decoded = decode_local_uri(uri.to_str()?.as_bytes())?;
        let decoded = String::from_utf8(decoded).ok()?;
        // A drive URI has an extra leading slash compared with a native path.
        // Reject root-relative paths so they cannot resolve on an arbitrary drive.
        let drive_path = decoded.strip_prefix('/')?;
        let bytes = drive_path.as_bytes();
        if bytes.len() < 3
            || !bytes[0].is_ascii_alphabetic()
            || bytes[1] != b':'
            || bytes[2] != b'/'
        {
            return None;
        }
        Some(PathBuf::from(drive_path.replace('/', "\\")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_uris_decode_once_and_accept_case_insensitive_scheme_and_host() {
        for uri in [
            b"file:///folder01/my%20file%23%25%2B%C3%A9".as_slice(),
            b"FILE://LOCALHOST/folder01/my%20file%23%25%2B%C3%A9",
            b"file:/folder01/my%20file%23%25%2B%C3%A9",
        ] {
            assert_eq!(
                decode_local_uri(uri).as_deref(),
                Some("/folder01/my file#%+é".as_bytes())
            );
        }
        assert_eq!(
            decode_local_uri(b"file:///folder01/%2520+file").as_deref(),
            Some(b"/folder01/%20+file".as_slice())
        );
    }

    #[test]
    fn unsupported_or_malformed_uris_are_rejected() {
        for uri in [
            "file://remote-host/folder01",
            "file://localhost",
            "file:folder01",
            "file:///folder01/%",
            "file:///folder01/%GG",
            "file:///folder01/%00",
            "file:///folder01?query",
            "file:///folder01#fragment",
            "remote:folder01",
        ] {
            assert!(decode_local_uri(uri.as_bytes()).is_none(), "{uri}");
        }
    }

    #[cfg(windows)]
    #[test]
    fn local_drive_uris_become_absolute_windows_paths() {
        assert_eq!(
            to_path(OsStr::new("file://localhost/C:/folder01/my%20file")),
            Some(PathBuf::from(r"C:\folder01\my file"))
        );
        assert!(to_path(OsStr::new("file:///folder01")).is_none());
        assert!(to_path(OsStr::new("file:///C:/folder01/%FF")).is_none());
    }

    #[cfg(unix)]
    #[test]
    fn unix_uris_preserve_non_utf8_filename_bytes() {
        use std::os::unix::ffi::OsStrExt;
        let path = to_path(OsStr::new("file:///folder01/my%20file%FF")).unwrap();
        assert_eq!(path.as_os_str().as_bytes(), b"/folder01/my file\xFF");
    }
}
