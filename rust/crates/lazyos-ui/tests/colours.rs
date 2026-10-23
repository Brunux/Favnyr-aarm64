//! Colours come from the design system, never from a widget.
//!
//! The grep test is the cheap half of the theme engine's contract (spec 004): a
//! widget that names `Color::Rgb(…)` cannot follow a second theme, and by the
//! time there are thirty of them nobody finds the one that did. The mapping
//! from the palette to the terminal library's type lives in `design.rs` and
//! nowhere else.

use std::fs;
use std::path::{Path, PathBuf};

/// Every `.rs` file under a directory, recursively.
fn sources(dir: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let Ok(entries) = fs::read_dir(dir) else {
        return found;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            found.extend(sources(&path));
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            found.push(path);
        }
    }
    found
}

#[test]
fn a_raw_colour_is_only_named_in_the_design_module() {
    let crates = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("the crate sits in rust/crates")
        .to_path_buf();
    let mut checked = 0;
    for name in ["lazyos-ui", "lazyos-cli"] {
        let dir = crates.join(name).join("src");
        for file in sources(&dir) {
            if file.file_name().is_some_and(|name| name == "design.rs") {
                continue;
            }
            let text = fs::read_to_string(&file).unwrap();
            assert!(
                !text.contains("Color::Rgb("),
                "{} names a raw colour: ask the Design for a ColorRole instead",
                file.display()
            );
            checked += 1;
        }
    }
    assert!(checked >= 5, "the walk found {checked} files");
}

#[test]
fn the_palette_is_the_only_colour_source() {
    // And the design module does the mapping once, from the theme's roles - not
    // from literals that would drift from `lazyos-theme`.
    let design = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("src/design.rs")
        .display()
        .to_string();
    let text = fs::read_to_string(&design).unwrap();
    assert!(
        text.contains("Color::Rgb(rgb.r, rgb.g, rgb.b)"),
        "one mapping function, from the theme's Rgb"
    );
    let hex_literals = text.matches("0x").count();
    assert!(
        hex_literals == 0,
        "design.rs carries {hex_literals} hex literals: the values belong to lazyos-theme"
    );
}
