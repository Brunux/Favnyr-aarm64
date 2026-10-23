//! Where the framework keeps its files, with XDG honoured.
//!
//! Nothing here creates a directory: `Escape returns 1 with nothing written to
//! disk` is a promise the framework keeps by *not having* a side effect to
//! undo, so a command asks for its path and only calls [`Paths::ensure`] when it
//! is about to write. The old scripts each had their own idea of where state
//! lived (`~/.cache/cliphist`, `~/.local/state/lazyos`, `$XDG_RUNTIME_DIR`) and
//! two of them disagreed; one resolver is the fix.

use std::collections::BTreeMap;
use std::io;
use std::path::PathBuf;

pub struct Paths {
    pub config: PathBuf,
    pub state: PathBuf,
    pub cache: PathBuf,
}

impl Paths {
    /// Resolve from `$HOME` and the XDG variables, without touching the disk.
    ///
    /// An empty variable is treated as unset, which is what the XDG base
    /// directory specification says (and what a `XDG_CACHE_HOME=` in a unit
    /// file actually means).
    pub fn resolve(vars: &BTreeMap<String, String>) -> io::Result<Self> {
        let get = |key: &str| vars.get(key).map(String::as_str).filter(|v| !v.is_empty());
        let home = get("HOME").ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                "HOME is not set: nowhere to keep the session's files",
            )
        })?;
        let dir = |xdg: &str, fallback: &str| match get(xdg) {
            Some(value) => PathBuf::from(value).join("lazyos"),
            None => PathBuf::from(home).join(fallback).join("lazyos"),
        };
        Ok(Self {
            config: dir("XDG_CONFIG_HOME", ".config"),
            state: dir("XDG_STATE_HOME", ".local/state"),
            cache: dir("XDG_CACHE_HOME", ".cache"),
        })
    }

    pub fn from_process() -> io::Result<Self> {
        let vars: BTreeMap<String, String> = std::env::vars().collect();
        Self::resolve(&vars)
    }

    /// Create the directories a command is about to write into.
    pub fn ensure(&self) -> io::Result<()> {
        std::fs::create_dir_all(&self.config)?;
        std::fs::create_dir_all(&self.state)?;
        std::fs::create_dir_all(&self.cache)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    #[test]
    fn paths_honour_xdg() {
        let paths = Paths::resolve(&env(&[
            ("HOME", "/home/brunux"),
            ("XDG_CONFIG_HOME", "/home/brunux/.config"),
            ("XDG_STATE_HOME", "/var/state/me"),
            ("XDG_CACHE_HOME", "/var/cache/me"),
        ]))
        .unwrap();
        assert_eq!(paths.config, PathBuf::from("/home/brunux/.config/lazyos"));
        assert_eq!(paths.state, PathBuf::from("/var/state/me/lazyos"));
        assert_eq!(paths.cache, PathBuf::from("/var/cache/me/lazyos"));
    }

    #[test]
    fn unset_or_empty_variables_fall_back_to_home() {
        let paths =
            Paths::resolve(&env(&[("HOME", "/home/brunux"), ("XDG_CONFIG_HOME", "")])).unwrap();
        assert_eq!(paths.config, PathBuf::from("/home/brunux/.config/lazyos"));
        assert_eq!(
            paths.state,
            PathBuf::from("/home/brunux/.local/state/lazyos")
        );
        assert_eq!(paths.cache, PathBuf::from("/home/brunux/.cache/lazyos"));
    }

    #[test]
    fn no_home_is_an_error_not_a_directory_in_pwd() {
        assert!(Paths::resolve(&env(&[])).is_err());
    }
}
