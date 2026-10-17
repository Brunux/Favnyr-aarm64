//! Bounded execution of optional local thumbnail and decompression tools.

use std::io::Read;
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

/// Command names shared by model previews and the archive recipes in the GUI.
/// The standalone Linux build is preferred because older p7zip variants may
/// lack the Zstandard decoder needed by compressed blend files.
pub const SEVEN_ZIP_PROGRAMS: &[&str] = &[
    "7zz",
    "7z",
    "7za",
    r"C:\Program Files\7-Zip\7z.exe",
    r"C:\Program Files (x86)\7-Zip\7z.exe",
];
/// How long an external preview or decompression tool may run before it is killed.
///
/// `Command::output()` waits without any bound: a crafted file can keep the
/// tool spinning forever, and the thumbnail queue runs several workers, so a
/// handful of such files would starve it entirely. Generous for real content ,
/// extracting one frame after an input seek takes well under a second.
pub(super) const TOOL_TIMEOUT: Duration = Duration::from_secs(10);

/// How often the bounded wait checks whether the tool has finished.
const TOOL_POLL: Duration = Duration::from_millis(25);

/// Runs `cmd` and returns its standard output, killing the child if it outlives
/// [`TOOL_TIMEOUT`]. `None` if it failed, was killed, or could not start.
///
/// The pipe is drained on its own thread. Polling the exit status while nobody
/// reads standard output would let a child fill its pipe buffer and block ,
/// indistinguishable from the hang this is meant to catch, and it would turn
/// every large frame into a false timeout.
pub(super) fn run_bounded(cmd: Command, limit: Duration) -> Option<Vec<u8>> {
    let output = run_bounded_output(cmd, limit, usize::MAX)?;
    output.complete.then_some(output.bytes)
}

pub(super) struct ToolOutput {
    pub(super) bytes: Vec<u8>,
    pub(super) complete: bool,
}

/// Reads at most `max_output` bytes. A capped output is returned as a prefix
/// so callers that only need an early file header can still use it. The child
/// is stopped immediately when the limit is reached.
pub(super) fn run_bounded_output(
    mut cmd: Command,
    limit: Duration,
    max_output: usize,
) -> Option<ToolOutput> {
    cmd.stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    let mut child = cmd.spawn().ok()?;
    let mut pipe = child.stdout.take()?;
    let capped = Arc::new(AtomicBool::new(false));
    let drain_capped = Arc::clone(&capped);
    let drain = std::thread::spawn(move || {
        let mut buffer = Vec::new();
        let mut chunk = [0u8; 8192];
        loop {
            let read = pipe.read(&mut chunk).ok()?;
            if read == 0 {
                return Some(buffer);
            }
            let remaining = max_output.saturating_sub(buffer.len());
            buffer.extend_from_slice(&chunk[..read.min(remaining)]);
            if read > remaining {
                drain_capped.store(true, Ordering::Release);
                return Some(buffer);
            }
        }
    });
    let deadline = Instant::now() + limit;
    let status = loop {
        if capped.load(Ordering::Acquire) {
            let _ = child.kill();
            let _ = child.wait();
            return Some(ToolOutput {
                bytes: drain.join().ok()??,
                complete: false,
            });
        }
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if Instant::now() >= deadline => {
                let _ = child.kill();
                let _ = child.wait();
                let _ = drain.join();
                return None;
            }
            Ok(None) => std::thread::sleep(TOOL_POLL),
            Err(_) => {
                let _ = child.kill();
                let _ = child.wait();
                let _ = drain.join();
                return None;
            }
        }
    };
    let bytes = drain.join().ok()??;
    status.success().then_some(ToolOutput {
        bytes,
        complete: !capped.load(Ordering::Acquire),
    })
}

/// On Windows, prevents a **console window** from opening (a "DOS" flash)
/// when launching an optional CLI from a GUI app. No-op elsewhere.
pub(super) fn no_console(cmd: &mut Command) {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    #[cfg(not(windows))]
    let _ = cmd;
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;

    #[test]
    fn a_tool_that_never_finishes_is_killed() {
        let started = Instant::now();
        let mut cmd = Command::new("sleep");
        cmd.arg("30");
        assert!(run_bounded(cmd, Duration::from_millis(200)).is_none());
        // Killed on the deadline rather than waited out.
        assert!(started.elapsed() < Duration::from_secs(5));
    }

    #[test]
    fn a_tool_that_finishes_in_time_returns_its_output() {
        let mut cmd = Command::new("echo");
        cmd.arg("ready");
        let out = run_bounded(cmd, Duration::from_secs(5)).expect("echo should succeed");
        assert_eq!(String::from_utf8_lossy(&out).trim(), "ready");
    }
}
