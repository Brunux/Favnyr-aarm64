//! `lazyos`, the session's multi-call binary.
//!
//! One binary answers to every name the session calls it by (`lazyos menu`,
//! `rofi-files`, …), decides where it should run, and owns the terminal and the
//! error story in one place. The commands themselves arrive in later specs; the
//! shape they plug into is this file.
//!
//! ```text
//! lazyos menu            the session menu     (spec 007: the real one)
//! lazyos menu files      the file browser     (spec 009: the real one)
//! lazyos --help          the table above, from the same data check-system reads
//! lazyos --version       the repository's VERSION, compiled in
//! ```
//!
//! Exit codes are the shell's contract (spec 001 §Design): `0` did something,
//! `1` Escape (nothing happened), `2` usage, `3` the action failed and the
//! reason was already shown - so `|| exit 0` in a binding stays correct.

pub mod command;
pub mod host;
pub mod paths;
pub mod term;
pub mod version;

use std::io::{self, Write};
use std::time::Duration;

use lazyos_theme::Palette;
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use ratatui::crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};

use command::{COMMANDS, ResolveError, Resolved};
use host::{Env, Launch};

pub const EXIT_OK: i32 = 0;
pub const EXIT_ESCAPE: i32 = 1;
pub const EXIT_USAGE: i32 = 2;
pub const EXIT_FAILED: i32 = 3;

/// Why a command could not do what it was asked, in the two lines the card
/// shows: the summary says what failed, the detail says what to do about it.
pub struct Failure {
    pub summary: String,
    pub detail: String,
}

impl Failure {
    pub fn new(summary: impl Into<String>, detail: impl Into<String>) -> Self {
        Self {
            summary: summary.into(),
            detail: detail.into(),
        }
    }
}

/// The whole process, with everything it reads injected so the tests can drive
/// it without a session.
pub fn run(argv: Vec<String>, env: &Env) -> i32 {
    let argv0 = argv
        .first()
        .cloned()
        .unwrap_or_else(|| "lazyos".to_string());
    let args: Vec<String> = argv.iter().skip(1).cloned().collect();

    if let Some(code) = global(&args) {
        return code;
    }

    let resolved = match command::resolve(&argv0, &args) {
        Ok(resolved) => resolved,
        Err(ResolveError::NoCommand) => {
            eprintln!("lazyos {}", version::version());
            eprintln!("usage: lazyos <command> [args]");
            eprintln!();
            print_commands(&mut io::stderr());
            return EXIT_USAGE;
        }
        Err(ResolveError::Unknown(name)) => {
            eprintln!("lazyos: unknown command '{name}'");
            print_commands(&mut io::stderr());
            return EXIT_USAGE;
        }
    };

    match host::plan(&resolved, env) {
        Launch::Inline => run_inline(&resolved),
        Launch::Spawn { program, args, .. } => match host::spawn(&program, &args) {
            Ok(()) => EXIT_OK,
            Err(error) => {
                eprintln!("lazyos: cannot open a window with {program}: {error}");
                eprintln!("lazyos: run it inside a terminal with LAZYOS_UI=1 to see the reason");
                EXIT_FAILED
            }
        },
        Launch::NoTerminal => {
            eprintln!("lazyos {}: no terminal to draw in", resolved.spec.path);
            eprintln!("lazyos: run it in a terminal, or from the session where `foot` exists");
            EXIT_FAILED
        }
    }
}

/// The flags that never resolve to a command: they are how a person (or
/// `check-system.sh`) reads the table without starting a UI.
fn global(args: &[String]) -> Option<i32> {
    let first = args.first().map(String::as_str)?;
    match first {
        "-V" | "--version" => {
            println!("{}", version::version());
            Some(EXIT_OK)
        }
        "-h" | "--help" => {
            let mut out = io::stdout();
            let _ = writeln!(out, "lazyos {} - the LazyOS session UI", version::version());
            let _ = writeln!(out);
            let _ = writeln!(out, "usage: lazyos <command> [args]");
            let _ = writeln!(out, "       <symlink name> [args]");
            let _ = writeln!(out);
            print_commands(&mut out);
            let _ = writeln!(out);
            let _ = writeln!(out, "options:");
            let _ = writeln!(out, "  -h, --help      this list");
            let _ = writeln!(
                out,
                "  -V, --version   the version this image was built with"
            );
            Some(EXIT_OK)
        }
        // Machine-readable, for the witness: the links the installer creates
        // today, from the same table `--help` prints.
        "--symlinks" => {
            let mut out = io::stdout();
            for (name, path) in command::installed_symlinks() {
                let _ = writeln!(out, "{name}\t{path}");
            }
            Some(EXIT_OK)
        }
        _ => None,
    }
}

fn print_commands(out: &mut impl Write) {
    let _ = writeln!(out, "commands:");
    for spec in COMMANDS {
        let _ = writeln!(out, "  {:<14} {}", spec.path, spec.summary);
    }
}

/// Draw in the terminal this process already has.
fn run_inline(resolved: &Resolved) -> i32 {
    let session = match term::Session::enter() {
        Ok(session) => session,
        Err(error) => {
            eprintln!("lazyos: cannot use this terminal: {error}");
            return EXIT_FAILED;
        }
    };
    let mut terminal = match Terminal::new(CrosstermBackend::new(io::stdout())) {
        Ok(terminal) => terminal,
        Err(error) => {
            eprintln!("lazyos: cannot size this terminal: {error}");
            return EXIT_FAILED;
        }
    };
    let palette = Palette::default();

    let code = match execute(resolved, &mut terminal, &palette) {
        Ok(code) => code,
        Err(failure) => {
            show_failure(&mut terminal, &palette, &failure);
            EXIT_FAILED
        }
    };

    // The terminal is restored by `session`'s Drop after `terminal` goes away.
    drop(session);
    code
}

fn execute(
    resolved: &Resolved,
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    palette: &Palette,
) -> Result<i32, Failure> {
    match resolved.spec.path {
        "menu" => menu(resolved, terminal, palette, "the session menu", "spec 007"),
        "menu files" => menu(resolved, terminal, palette, "the file browser", "spec 009"),
        other => Err(Failure::new(
            format!("`{other}` is in the table but has no implementation"),
            "this is a bug: the command table and this match disagree",
        )),
    }
}

/// The one command spec 001 ships: it proves the host, the frame and the keys.
///
/// It draws, it closes on Escape and it writes nothing - no state directory is
/// created, which is the test for "a menu is a process, not a daemon".
fn menu(
    resolved: &Resolved,
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    palette: &Palette,
    title: &str,
    arrives: &str,
) -> Result<i32, Failure> {
    if let Some(extra) = resolved.args.first() {
        return Err(Failure::new(
            format!("`{extra}` is not something this menu knows"),
            format!("try: lazyos {}", resolved.spec.path),
        ));
    }
    let hint = format!("{arrives} - Esc closes");

    loop {
        terminal
            .draw(|frame| {
                // A panic here is the acceptance test's way of proving the hook
                // restores the cursor and the alternate screen.
                if std::env::var_os("LAZYOS_FORCE_PANIC").is_some() {
                    panic!("LAZYOS_FORCE_PANIC: the frame's panic path");
                }
                lazyos_ui::empty_state(frame, frame.area(), palette, title, &hint);
            })
            .map_err(|error| Failure::new("cannot draw this terminal", error.to_string()))?;

        match event::read() {
            Ok(Event::Key(key)) if key.kind == KeyEventKind::Press => match key.code {
                KeyCode::Esc | KeyCode::Char('q') => return Ok(EXIT_ESCAPE),
                KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    return Ok(EXIT_ESCAPE);
                }
                _ => {}
            },
            Ok(_) => {}
            Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
            Err(error) => {
                return Err(Failure::new(
                    "the terminal stopped answering",
                    error.to_string(),
                ));
            }
        }
    }
}

/// The error card: what failed, and the one thing to try next.
///
/// It waits for a key because a card that closes itself is a card nobody read,
/// but never for ever - a broken input must not be able to hang the session.
fn show_failure(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    palette: &Palette,
    failure: &Failure,
) {
    let _ = terminal.draw(|frame| {
        lazyos_ui::error_card(
            frame,
            frame.area(),
            palette,
            &failure.summary,
            &failure.detail,
        )
    });
    if matches!(event::poll(Duration::from_secs(30)), Ok(true)) {
        let _ = event::read();
    }
}
