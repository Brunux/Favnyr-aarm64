//! The engine against the surfaces it ships: every theme, every file, twice.
//!
//! Three properties, each the witness of a promise in `specs/004-theme-engine.md`:
//!
//! 1. **Every surface renders for every built-in theme** and leaves no
//!    placeholder behind, no colour that is not one of the theme's own (a
//!    literal would survive a switch that every other line followed), and its
//!    header on the first line (that is how a hand edit is told from a file the
//!    engine owns).
//! 2. **The golden files are current.** `tests/fixtures/themes/<slug>/<surface>`
//!    is what the engine produced when the template was last edited;
//!    `UPDATE_SNAPSHOTS=1 cargo test -p lazyos-theme` rewrites them, the same
//!    convention the frame snapshots use.
//! 3. **A second `theme set` is an empty diff**, and switching away and back
//!    restores byte-identical files - so the idempotence is a property of the
//!    engine, not of a cache.

use std::fs;
use std::path::{Path, PathBuf};

use lazyos_theme::store::{self, Paths};
use lazyos_theme::{SURFACES, Theme, builtin};

fn theme(slug: &str) -> Theme {
    let text = builtin::get(slug).unwrap_or_else(|| panic!("{slug} is built in"));
    Theme::parse(text).unwrap_or_else(|error| panic!("{slug}: {error}"))
}

fn fixture(slug: &str, surface: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/themes")
        .join(slug)
        .join(surface)
}

struct Temp(PathBuf);

impl Temp {
    fn new(tag: &str) -> Self {
        let path = std::env::temp_dir().join(format!(
            "lazyos-theme-surfaces-{tag}-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).unwrap();
        Self(path)
    }

    fn paths(&self) -> Paths {
        Paths::new(self.0.join("home"), self.0.join("state"))
    }
}

impl Drop for Temp {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn every_surface_renders_for_every_theme_without_a_literal_or_a_placeholder() {
    for (slug, _) in builtin::ALL {
        let theme = theme(slug);
        let allowed: Vec<String> = theme.table().values().map(|c| c.hashed()).collect();
        for surface in &SURFACES {
            let body = store::render_surface(surface, &theme, slug, "/home/lazy")
                .unwrap_or_else(|error| panic!("{slug}/{name}: {error}", name = surface.name));
            assert!(
                !body.contains("{{"),
                "{}/{name}: an unresolved placeholder survived",
                slug,
                name = surface.name
            );
            for line in body.lines() {
                if let Some(at) = line.find('#') {
                    let rest = &line[at..];
                    let literal: String = rest.chars().take(7).collect();
                    if literal.len() == 7
                        && literal[1..].chars().all(|c| c.is_ascii_hexdigit())
                        && !literal[1..].chars().all(|c| c == '0')
                    {
                        assert!(
                            allowed.iter().any(|colour| colour == &literal),
                            "{}/{name}: {literal} is not one of the theme's colours, so a \
                             literal survived in the template: {line}",
                            slug,
                            name = surface.name
                        );
                    }
                }
            }
            assert!(
                body.lines()
                    .next()
                    .is_some_and(|line| line.contains("LazyOS theme:")),
                "{}/{name}: the header belongs on the first line",
                slug,
                name = surface.name
            );
        }
    }
}

#[test]
fn the_golden_files_are_current() {
    let update = std::env::var("UPDATE_SNAPSHOTS").is_ok();
    let mut stale = Vec::new();
    for (slug, _) in builtin::ALL {
        let theme = theme(slug);
        for surface in &SURFACES {
            let body = store::render_surface(surface, &theme, slug, "/home/lazy").unwrap();
            let path = fixture(slug, surface.name);
            if update {
                fs::create_dir_all(path.parent().unwrap()).unwrap();
                fs::write(&path, &body).unwrap();
                continue;
            }
            match fs::read_to_string(&path) {
                Ok(golden) if golden == body => {}
                Ok(_) => stale.push(format!("{slug}/{} (differs)", surface.name)),
                Err(_) => stale.push(format!("{slug}/{} (missing)", surface.name)),
            }
        }
    }
    assert!(
        stale.is_empty(),
        "the golden files are stale: {}. Regenerate with `UPDATE_SNAPSHOTS=1 cargo test -p lazyos-theme`",
        stale.join(", ")
    );
}

#[test]
fn a_second_set_is_an_empty_diff_and_a_round_trip_is_byte_identical() {
    let temp = Temp::new("idempotent");
    let paths = temp.paths();
    let solarized = theme("solarized");

    for surface in &SURFACES {
        store::install(&paths, surface, &solarized, "solarized", false).unwrap();
    }
    let first: Vec<String> = SURFACES
        .iter()
        .map(|surface| fs::read_to_string(paths.target(surface)).unwrap())
        .collect();

    for surface in &SURFACES {
        assert_eq!(
            store::install(&paths, surface, &solarized, "solarized", false).unwrap(),
            store::Outcome::Unchanged,
            "{} was rewritten although nothing changed",
            surface.name
        );
    }

    let gruvbox = theme("gruvbox");
    for surface in &SURFACES {
        assert_eq!(
            store::install(&paths, surface, &gruvbox, "gruvbox", false).unwrap(),
            store::Outcome::Written
        );
    }
    for surface in &SURFACES {
        store::install(&paths, surface, &solarized, "solarized", false).unwrap();
    }
    let back: Vec<String> = SURFACES
        .iter()
        .map(|surface| fs::read_to_string(paths.target(surface)).unwrap())
        .collect();
    assert_eq!(first, back, "switching away and back restores every byte");
}

#[test]
fn every_surface_changes_with_the_theme_or_it_is_not_reading_the_palette() {
    let solarized = theme("solarized");
    let mocha = theme("catppuccin");
    for surface in &SURFACES {
        let a = store::render_surface(surface, &solarized, "solarized", "/home/lazy").unwrap();
        let b = store::render_surface(surface, &mocha, "catppuccin", "/home/lazy").unwrap();
        assert_ne!(
            a, b,
            "{} is identical in two themes: a colour is hardcoded",
            surface.name
        );
    }
}
