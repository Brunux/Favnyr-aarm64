//! Where a command runs: this terminal, a new `foot` window, or nowhere.
//!
//! The rule is one sentence and it is enforced in one place, because a guard
//! that lives in each command is a guard that one command will forget: **a
//! command that already has a terminal runs in it; one that does not gets a
//! `foot` window; one with neither says so.**
//!
//! `LAZYOS_UI=1` is how a child knows it is the UI (the host sets it in the
//! environment of the `foot` it spawns), so a nested invocation can never open
//! a second window - it runs inline instead.

use std::collections::BTreeMap;
use std::io;
use std::path::Path;
use std::process::{Command, Stdio};

use crate::command::Resolved;

/// Everything the host decides with, in one injectable value so the tests do
/// not depend on the machine they run on.
pub struct Env {
    vars: BTreeMap<String, String>,
    /// `stdout` is a terminal (what "I can draw here" needs).
    pub is_tty: bool,
    pub have_foot: bool,
    /// The binary's own path, which the child must be given instead of the
    /// symlink name it was called by.
    pub exe: String,
}

impl Env {
    pub fn new(
        vars: BTreeMap<String, String>,
        is_tty: bool,
        have_foot: bool,
        exe: impl Into<String>,
    ) -> Self {
        Self {
            vars,
            is_tty,
            have_foot,
            exe: exe.into(),
        }
    }

    pub fn from_process(argv0: &str) -> Self {
        use std::io::IsTerminal;
        let vars: BTreeMap<String, String> = std::env::vars().collect();
        let path = vars.get("PATH").map(String::as_str).unwrap_or("");
        Self {
            is_tty: io::stdout().is_terminal(),
            have_foot: on_path(path, "foot"),
            exe: std::env::current_exe()
                .map(|p| p.display().to_string())
                .unwrap_or_else(|_| argv0.to_string()),
            vars,
        }
    }

    pub fn get(&self, key: &str) -> Option<&str> {
        self.vars
            .get(key)
            .map(String::as_str)
            .filter(|v| !v.is_empty())
    }

    /// `LAZYOS_UI=1`, `true` or `yes` all mean the same thing: this process is
    /// the UI and must not spawn another.
    pub fn flag(&self, key: &str) -> bool {
        matches!(self.get(key), Some("1") | Some("true") | Some("yes"))
    }
}

/// What the host will do with a resolved command.
#[derive(Debug, PartialEq, Eq)]
pub enum Launch {
    /// Draw in the terminal this process already has.
    Inline,
    /// Spawn the window and return; the child draws.
    Spawn {
        program: String,
        args: Vec<String>,
        app_id: String,
    },
    /// No terminal and no compositor to ask for one.
    NoTerminal,
    /// Run in place: the command owns no surface (it writes files).
    Direct,
}

/// Decide, from the environment alone. Pure: the tests call it directly.
pub fn plan(resolved: &Resolved, env: &Env) -> Launch {
    // A direct command has no surface to draw on, so neither the guard nor the
    // compositor has anything to say about it.
    if resolved.spec.shape == crate::command::Shape::Direct {
        return Launch::Direct;
    }
    // The guard: a process that is already the UI never spawns the UI. If it
    // also has no terminal, that is a failure the card reports - not a second
    // window nobody asked for.
    if env.flag("LAZYOS_UI") {
        return if env.is_tty {
            Launch::Inline
        } else {
            Launch::NoTerminal
        };
    }
    if env.get("WAYLAND_DISPLAY").is_some() && env.have_foot {
        return Launch::Spawn {
            program: "foot".into(),
            app_id: resolved.spec.shape.app_id(),
            args: window_args(resolved, &env.exe),
        };
    }
    if env.is_tty {
        return Launch::Inline;
    }
    Launch::NoTerminal
}

/// The argv the window gets: the canonical words, never the symlink that
/// started us (`rofi-files` becomes `menu files`).
///
/// The size is foot's own `--window-size-pixels` rather than a `resize set`
/// from sway: the compositor applies its rule at map time, and foot then commits
/// its preferred size over it (measured: a 900x600 rule produced a 700x496
/// window, but a 900-wide centre). Asking foot for the pixels is deterministic
/// and needs nobody to win a race.
fn window_args(resolved: &Resolved, exe: &str) -> Vec<String> {
    let mut args = vec![format!("--app-id={}", resolved.spec.shape.app_id())];
    if let Some((width, height)) = resolved.spec.shape.size() {
        args.push(format!("--window-size-pixels={width}x{height}"));
    }
    args.push("-e".to_string());
    args.push(exe.to_string());
    args.extend(resolved.words().into_iter().map(str::to_string));
    args.extend(resolved.args.iter().cloned());
    args
}

/// Start the window and return without waiting for it: the caller is a key
/// binding, and blocking it on a UI session is how a binding starts to feel
/// broken.
pub fn spawn(program: &str, args: &[String]) -> io::Result<()> {
    let mut child = Command::new(program)
        .args(args)
        .env("LAZYOS_UI", "1")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        // stderr stays connected: a window that fails to open is the one error
        // nobody sees if this is silenced, and the journal is where it belongs.
        .spawn()?;
    // One non-blocking reap attempt: the parent exits immediately afterwards,
    // so this only covers the case where foot died instantly (a missing
    // WAYLAND_DISPLAY, a bad flag) and keeps it from being a zombie in tests.
    let _ = child.try_wait();
    Ok(())
}

fn on_path(path: &str, name: &str) -> bool {
    path.split(':')
        .filter(|dir| !dir.is_empty())
        .any(|dir| Path::new(dir).join(name).is_file())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env(pairs: &[(&str, &str)], is_tty: bool, foot: bool) -> Env {
        Env::new(
            pairs
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
            is_tty,
            foot,
            "/usr/bin/lazyos",
        )
    }

    fn menu() -> Resolved {
        Resolved {
            spec: &crate::command::COMMANDS[0],
            args: vec![],
        }
    }

    #[test]
    fn host_guard_does_not_nest() {
        // The child: LAZYOS_UI is set, a compositor and foot are available, and
        // it still must not spawn a second window.
        let child = env(
            &[("LAZYOS_UI", "1"), ("WAYLAND_DISPLAY", "wayland-0")],
            true,
            true,
        );
        assert_eq!(plan(&menu(), &child), Launch::Inline);

        // The same process without a terminal is a failure, not a window.
        let child_no_tty = env(
            &[("LAZYOS_UI", "1"), ("WAYLAND_DISPLAY", "wayland-0")],
            false,
            true,
        );
        assert_eq!(plan(&menu(), &child_no_tty), Launch::NoTerminal);
    }

    #[test]
    fn from_sway_it_opens_one_window_with_the_canonical_command() {
        let sway = env(&[("WAYLAND_DISPLAY", "wayland-0")], false, true);
        let launch = plan(&menu(), &sway);
        let Launch::Spawn {
            program,
            args,
            app_id,
        } = launch
        else {
            panic!("from sway the host must spawn a window, got {launch:?}");
        };
        assert_eq!(program, "foot");
        assert_eq!(app_id, "lazyos-pick");
        assert_eq!(args[0], "--app-id=lazyos-pick");
        assert_eq!(args[1], "--window-size-pixels=900x600");
        assert_eq!(args[2], "-e");
        assert_eq!(args[3], "/usr/bin/lazyos");
        assert_eq!(&args[4..], &["menu"]);

        // A symlink that resolves to `menu files` hands over the words, not the
        // name it was called by.
        let files = Resolved {
            spec: crate::command::COMMANDS
                .iter()
                .find(|spec| spec.path == "menu files")
                .expect("menu files is in the table"),
            args: vec!["--search".into()],
        };
        let Launch::Spawn { args, .. } = plan(&files, &sway) else {
            panic!("expected a window");
        };
        assert_eq!(&args[4..], &["menu", "files", "--search"]);
    }

    #[test]
    fn a_terminal_that_is_already_there_is_used() {
        let shell = env(&[], true, false);
        assert_eq!(plan(&menu(), &shell), Launch::Inline);
    }

    #[test]
    fn no_terminal_and_no_compositor_is_reported() {
        let bare = env(&[], false, false);
        assert_eq!(plan(&menu(), &bare), Launch::NoTerminal);
    }

    #[test]
    fn wayland_without_foot_still_uses_a_terminal_if_there_is_one() {
        let ssh = env(&[("WAYLAND_DISPLAY", "wayland-0")], true, false);
        assert_eq!(plan(&menu(), &ssh), Launch::Inline);
    }
}
