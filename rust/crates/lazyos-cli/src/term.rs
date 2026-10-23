//! Putting the terminal back.
//!
//! A menu is a process that borrows a terminal: it turns off echo, goes to the
//! alternate screen, hides the cursor and enables mouse tracking. Every one of
//! those is invisible until it is *not* undone, and then the shell is unusable -
//! which is why this module owns the sequence instead of each command doing it
//! in its own `Drop`.
//!
//! Four exits have to be covered, and the code differs on purpose:
//!
//! | exit | who restores |
//! |---|---|
//! | normal / Escape | `Drop` for [`Session`] |
//! | panic | the hook installed by [`Session::enter`] |
//! | SIGINT/SIGTERM/SIGHUP | [`on_signal`], which may only use async-signal-safe calls |
//! | SIGKILL | nobody can: the kernel does not run user code, so nothing is promised |
//!
//! The signal path restores the *termios* too (`tcsetattr` is async-signal-safe,
//! `write` is, `_exit` is), because leaving a pty in raw mode for the shell that
//! inherits it is the same bug in another costume.

use std::io::{self, Write};
use std::sync::Once;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicI32, Ordering};

use ratatui::crossterm::cursor;
use ratatui::crossterm::execute;
use ratatui::crossterm::terminal::{EnterAlternateScreen, disable_raw_mode, enable_raw_mode};

/// The bytes that undo everything the UI turns on, in the reverse order it was
/// turned on. Mouse tracking is disabled first (a stale `?1003h` makes every
/// pointer move a byte on the shell's stdin), then colours, then the cursor,
/// then the screen.
pub const RESTORE: &str = "\x1b[?1006l\x1b[?1003l\x1b[?1002l\x1b[?1000l\
                           \x1b[0m\x1b[?25h\x1b[?1049l";

static SAVED_TERMIOS: OnceLock<libc::termios> = OnceLock::new();
static TTY_FD: AtomicI32 = AtomicI32::new(-1);

/// A terminal in UI mode. Dropping it restores the terminal.
pub struct Session {
    _private: (),
}

impl Session {
    /// Take the terminal: raw mode, alternate screen, hidden cursor.
    ///
    /// The hooks are installed before the terminal is touched, so a panic
    /// during the setup itself still restores what had already changed.
    pub fn enter() -> io::Result<Self> {
        install_hooks();
        enable_raw_mode()?;
        execute!(io::stdout(), EnterAlternateScreen, cursor::Hide)?;
        Ok(Self { _private: () })
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        restore();
    }
}

/// Put the terminal back. Safe to call twice (the sequences are idempotent).
pub fn restore() {
    // Termios first: if anything below fails, echo is still back.
    let _ = disable_raw_mode();
    let mut out = io::stdout();
    let _ = out.write_all(RESTORE.as_bytes());
    let _ = out.flush();
}

fn install_hooks() {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        // Save what the shell handed us before crossterm changes it, so the
        // signal path can put it back with an async-signal-safe call.
        let mut saved: libc::termios = unsafe { std::mem::zeroed() };
        if unsafe { libc::tcgetattr(libc::STDIN_FILENO, &mut saved) } == 0 {
            let _ = SAVED_TERMIOS.set(saved);
            TTY_FD.store(libc::STDIN_FILENO, Ordering::Relaxed);
        }

        let previous = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            // A panic message printed on the alternate screen disappears with
            // it, so restore first and let the default hook print on the shell.
            restore();
            previous(info);
        }));

        for signal in [libc::SIGINT, libc::SIGTERM, libc::SIGHUP] {
            unsafe {
                libc::signal(signal, on_signal as *const () as libc::sighandler_t);
            }
        }
    });
}

/// The signal path: termios, the restore sequence, and out.
///
/// Only async-signal-safe calls are allowed here (`tcsetattr`, `write`,
/// `_exit`); an allocation or a lock would be a deadlock waiting for the signal
/// that arrived while the allocator held its lock.
extern "C" fn on_signal(signal: libc::c_int) {
    if let Some(saved) = SAVED_TERMIOS.get() {
        let fd = TTY_FD.load(Ordering::Relaxed);
        if fd >= 0 {
            unsafe {
                libc::tcsetattr(fd, libc::TCSANOW, saved);
            }
        }
    }
    unsafe {
        libc::write(libc::STDOUT_FILENO, RESTORE.as_ptr().cast(), RESTORE.len());
        libc::_exit(128 + signal);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_restore_sequence_undoes_everything_in_the_right_order() {
        let undo_mouse = RESTORE.find("1003l").expect("mouse motion is turned off");
        let undo_screen = RESTORE.find("1049l").expect("the alternate screen is left");
        let show_cursor = RESTORE.find("25h").expect("the cursor comes back");
        assert!(
            undo_mouse < show_cursor,
            "mouse off before the cursor shows"
        );
        assert!(show_cursor < undo_screen, "the screen is left last");
        assert!(RESTORE.starts_with("\x1b["), "it is a terminal sequence");
    }

    #[test]
    fn the_restore_sequence_does_not_enable_anything() {
        // A restore that turns something *on* is how a terminal ends up with a
        // hidden cursor after a crash: only the `l` (off) forms may appear.
        assert!(!RESTORE.contains("1003h"));
        assert!(!RESTORE.contains("1049h"));
        assert!(!RESTORE.contains("?25l"));
    }
}
