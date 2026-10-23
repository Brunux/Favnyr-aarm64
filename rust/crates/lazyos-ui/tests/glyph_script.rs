//! The alphabet table is a build artifact with a script behind it, so the script
//! is tested the way it is used.
//!
//! One test runs `scripts/make-glyphs.sh` against a fixture character map and a
//! fixture "font", which is what makes the filter observable without depending
//! on the fonts of the machine running the tests: the map below carries every
//! candidate except `U+EA85` (the terminal), so the generated table must give
//! `Folder` a codepoint and `Terminal` its text fallback, and the pin must count
//! both.  The other test runs the same script with `--check` against the real
//! font, and skips where there is none - a runner without
//! `fonts-nerd-symbols` has nothing to check against, and pretending otherwise
//! would make CI red for a package the tests do not need.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(3)
        .expect("the crate lives three levels below the repository root")
        .to_path_buf()
}

fn script() -> PathBuf {
    repo_root().join("scripts/make-glyphs.sh")
}

/// A scratch directory that does not collide with a parallel test.
fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("lazyos-glyphs-{}-{name}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("a scratch directory");
    dir
}

/// The font `fonts-nerd-symbols` installs, when this machine has it.
fn symbol_font() -> Option<PathBuf> {
    let out = Command::new("fc-match")
        .args(["-f", "%{file}", "Symbols Nerd Font Mono"])
        .output()
        .ok()?;
    let path = PathBuf::from(String::from_utf8(out.stdout).ok()?);
    let name = path.file_name()?.to_string_lossy().into_owned();
    name.starts_with("SymbolsNerdFont").then_some(path)
}

/// Count `needle` in `text`, for the invariants below.
fn count(text: &str, needle: &str) -> usize {
    text.matches(needle).count()
}

#[test]
fn the_generator_drops_exactly_the_codepoint_the_font_lacks() {
    let dir = scratch("fixture");
    // Every range a candidate needs except U+EA85: `ea60-ea84` stops just
    // before the terminal and `ea86-ea88` starts just after it.
    fs::write(
        dir.join("charset"),
        "23fb-23fe e000-e00a e0a0-e0a3 e0b0-e0c8 e0ca e0cc-e0d2 e0d4 e0d6-e0d7 \
         e5fa-e6b8 e700-e8ef ea60-ea84 ea86-ea88 ea8a-ea8c ea8f-eac7 eac9 \
         eacc-eb09 eb0b-eb4e eb50-ec1e ee00-ee0b f400-f533",
    )
    .expect("the fixture character map");
    fs::write(dir.join("font.ttf"), b"not a font, a pin\n").expect("the fixture font");

    let status = Command::new("sh")
        .arg(script())
        .args(["--font", dir.join("font.ttf").to_str().unwrap()])
        .args(["--charset", dir.join("charset").to_str().unwrap()])
        .args(["--out", dir.join("table.rs").to_str().unwrap()])
        .args(["--sidecar", dir.join("font.pin").to_str().unwrap()])
        .status()
        .expect("scripts/make-glyphs.sh runs");
    assert!(status.success(), "the generator failed on the fixture");

    let table = fs::read_to_string(dir.join("table.rs")).expect("the generated table");
    let pin = fs::read_to_string(dir.join("font.pin")).expect("the generated pin");

    assert!(
        table.is_ascii(),
        "the generated table carries a literal private-use character"
    );
    assert!(
        table.contains("glyph: Glyph::Folder,\n        draw: Some(\"\\u{ea83}\")"),
        "Folder lost the codepoint the fixture carries"
    );
    assert!(
        table.contains("glyph: Glyph::Terminal,\n        draw: None"),
        "Terminal kept a codepoint the fixture does not carry"
    );
    assert!(
        table.contains("glyph: Glyph::Image,\n        draw: Some(\"\\u{f4e5}\")"),
        "Image lost its codepoint"
    );

    // The pin counts what the table actually holds, and the two have to add up
    // to one entry per name: a table with a missing entry is a name that would
    // draw nothing.
    // Count the fields, not the prose: the header names both forms too.
    let entries = count(&table, "\n        glyph: Glyph::");
    let drawn = count(&table, "\n        draw: Some(");
    let fallback = count(&table, "\n        draw: None");
    assert_eq!(drawn + fallback, entries, "an entry has no draw field");
    assert!(
        pin.contains(&format!("drawn={drawn}\n")),
        "the pin disagrees with the table: {pin}"
    );
    assert!(
        pin.contains(&format!("fallback={fallback}\n")),
        "the pin disagrees with the table: {pin}"
    );
    assert!(
        fallback >= 8,
        "the eight Font Awesome-only names are not in the fixture's table"
    );

    // Idempotent: a second run writes the same bytes.
    let again = fs::read_to_string(dir.join("table.rs")).unwrap();
    let status = Command::new("sh")
        .arg(script())
        .args(["--font", dir.join("font.ttf").to_str().unwrap()])
        .args(["--charset", dir.join("charset").to_str().unwrap()])
        .args(["--out", dir.join("table.rs").to_str().unwrap()])
        .args(["--sidecar", dir.join("font.pin").to_str().unwrap()])
        .status()
        .expect("the generator runs a second time");
    assert!(status.success());
    assert_eq!(
        again,
        fs::read_to_string(dir.join("table.rs")).unwrap(),
        "two runs of the generator disagree"
    );

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn the_committed_table_is_what_the_generator_writes() {
    let Some(_font) = symbol_font() else {
        eprintln!("skipped: no Symbols Nerd Font on this host (fonts-nerd-symbols)");
        return;
    };
    let out = Command::new("sh")
        .arg(script())
        .arg("--check")
        .output()
        .expect("scripts/make-glyphs.sh --check runs");
    assert!(
        out.status.success(),
        "the committed table is stale: {}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
}
