//! The TUI alphabet: one Nerd Font codepoint per name a window draws.
//!
//! A row's picture is a Nerd Font codepoint, not a Font Awesome one.  The
//! alphabet belongs to the terminal (spec 003) and the launcher's images belong
//! to rofi: a command names a [`Glyph`], never a codepoint, so the picture is
//! one word in the source and one table in the build - `scripts/make-glyphs.sh`
//! writes `glyphs/table.rs` from the Nerd Fonts names, filtered against the
//! character map of the font that has to draw it (`fonts-nerd-symbols`).
//!
//! Two rules come out of that filter:
//!
//! - **a name the symbol package does not draw shows text, never a box.**
//!   `fonts-nerd-symbols` is DFSG-stripped: it carries the Octicons, Codicons,
//!   Devicons, Seti and Powerline sets and none of Font Awesome or the MDI
//!   plane, so `Bluetooth`, `Wifi`, `Battery`, `Keyboard`, `Mouse`,
//!   `Calculator`, `Qr` and `Crop` have no codepoint at all and draw their
//!   short token instead (`bt`, `wifi`, `bat`, `kbd`, `pad`, `=`, `qr`,
//!   `crop`).  A box is a hole in the row that logs nothing, and that is the
//!   failure this module exists to make impossible.
//! - **the codepoints stay escaped in the source** (`"\u{ea85}"`): a literal
//!   private-use character does not survive every editor, and this repository
//!   has already shipped a row of empty spans that way.
//!
//! The glyph column is the list's, not the glyph's: `ListState` reserves the
//! widest cell among its rows, so one text token does not shift the labels
//! beside it.
//!
//! `lazyos --glyphs` prints the codepoints the windows draw (the frame's from
//! [`crate::design::GLYPHS`] and these) and `lazyos --glyph-font` prints the
//! pin; `scripts/check-system.sh` compares the pin with the font installed on
//! the machine and asks that font for every codepoint in one `fc-query` call.

mod table;

pub use table::{DRAWN, FALLBACK, FONT_FAMILY, FONT_FILE, FONT_SHA256, NERD_VERSION};

/// One name in the alphabet.
///
/// A variant is a handle, not data: the codepoint lives in the generated table,
/// and regenerating that table is how a picture changes.  The variant set is
/// what a command can ask for, which is why it is written by hand and the
/// codepoints are not.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Glyph {
    // files and places
    Folder,
    File,
    Document,
    Pdf,
    Html,
    Archive,
    Executable,
    Hidden,
    BrokenLink,
    Image,
    Video,
    Music,
    Library,
    Home,
    Desktop,
    Terminal,

    // the machine
    Window,
    Disk,
    Gear,
    Book,
    People,
    Globe,
    Link,
    Volume,
    Bell,
    BellOff,
    Clock,
    Calendar,
    History,
    Trash,
    Bug,
    Mic,
    Sliders,
    Lock,
    Unlock,
    Eye,
    Moon,
    Brightness,
    Keyboard,
    Wifi,
    Bluetooth,
    Battery,
    Mouse,
    Network,
    Server,
    Database,
    Chip,
    Display,
    Power,
    Logout,
    Reboot,
    Rotate,

    // actions
    Search,
    Warning,
    Info,
    Check,
    Undo,
    Save,
    Sync,
    Download,
    Capture,
    Camera,
    Crop,
    Qr,
    Clipboard,
    Smile,
    Filter,
    Tag,
    Key,
    Map,
    Theme,
    Brush,
    Box,
    Calculator,
    Play,
    Pause,
    Stop,
    Next,
    Prev,
    ArrowUp,
    ArrowDown,
    ArrowLeft,
    ArrowRight,
    Target,
    Star,
    Heart,
    Pin,
    Lightbulb,
    Cloud,

    // tools and agents
    Code,
    Agent,
    Sparkle,
    Git,
    Github,
    Docker,
    Rust,
    Python,
    Node,
    Linux,
    Debian,
    Shield,
}

/// Every variant, in the order the generated table lists them.
///
/// A new variant goes here **and** in `scripts/make-glyphs.sh`;
/// `every_variant_has_an_entry_and_the_table_adds_nothing` fails when the two
/// disagree, because a name with no entry would draw nothing at all.
pub const VARIANTS: &[Glyph] = &[
    Glyph::Folder,
    Glyph::File,
    Glyph::Document,
    Glyph::Pdf,
    Glyph::Html,
    Glyph::Archive,
    Glyph::Executable,
    Glyph::Hidden,
    Glyph::BrokenLink,
    Glyph::Image,
    Glyph::Video,
    Glyph::Music,
    Glyph::Library,
    Glyph::Home,
    Glyph::Desktop,
    Glyph::Terminal,
    Glyph::Window,
    Glyph::Disk,
    Glyph::Gear,
    Glyph::Book,
    Glyph::People,
    Glyph::Globe,
    Glyph::Link,
    Glyph::Volume,
    Glyph::Bell,
    Glyph::BellOff,
    Glyph::Clock,
    Glyph::Calendar,
    Glyph::History,
    Glyph::Trash,
    Glyph::Bug,
    Glyph::Mic,
    Glyph::Sliders,
    Glyph::Lock,
    Glyph::Unlock,
    Glyph::Eye,
    Glyph::Moon,
    Glyph::Brightness,
    Glyph::Keyboard,
    Glyph::Wifi,
    Glyph::Bluetooth,
    Glyph::Battery,
    Glyph::Mouse,
    Glyph::Network,
    Glyph::Server,
    Glyph::Database,
    Glyph::Chip,
    Glyph::Display,
    Glyph::Power,
    Glyph::Logout,
    Glyph::Reboot,
    Glyph::Rotate,
    Glyph::Search,
    Glyph::Warning,
    Glyph::Info,
    Glyph::Check,
    Glyph::Undo,
    Glyph::Save,
    Glyph::Sync,
    Glyph::Download,
    Glyph::Capture,
    Glyph::Camera,
    Glyph::Crop,
    Glyph::Qr,
    Glyph::Clipboard,
    Glyph::Smile,
    Glyph::Filter,
    Glyph::Tag,
    Glyph::Key,
    Glyph::Map,
    Glyph::Theme,
    Glyph::Brush,
    Glyph::Box,
    Glyph::Calculator,
    Glyph::Play,
    Glyph::Pause,
    Glyph::Stop,
    Glyph::Next,
    Glyph::Prev,
    Glyph::ArrowUp,
    Glyph::ArrowDown,
    Glyph::ArrowLeft,
    Glyph::ArrowRight,
    Glyph::Target,
    Glyph::Star,
    Glyph::Heart,
    Glyph::Pin,
    Glyph::Lightbulb,
    Glyph::Cloud,
    Glyph::Code,
    Glyph::Agent,
    Glyph::Sparkle,
    Glyph::Git,
    Glyph::Github,
    Glyph::Docker,
    Glyph::Rust,
    Glyph::Python,
    Glyph::Node,
    Glyph::Linux,
    Glyph::Debian,
    Glyph::Shield,
];

impl Glyph {
    /// Every name in the alphabet, in table order.
    pub fn all() -> &'static [Glyph] {
        VARIANTS
    }

    /// The codepoint the symbol font draws, or `None` when it does not carry
    /// this name at all.
    pub fn codepoint(self) -> Option<char> {
        self.draw().and_then(|draw| draw.chars().next())
    }

    /// What the glyph column shows: the codepoint, or the short text token for
    /// a name the symbol set does not draw.  Never empty, never a box.
    pub fn cell(self) -> &'static str {
        match self.draw() {
            Some(draw) => draw,
            None => {
                note_text_fallback(self);
                self.token()
            }
        }
    }

    /// The short text a name without a codepoint draws.
    ///
    /// Eight of them are Font Awesome names the stripped symbol set leaves out;
    /// the rest answer their own name, which is what a codepoint dropped by a
    /// font upgrade falls back to (`scripts/make-glyphs.sh` turns it into
    /// `draw: None` and nothing else changes).
    pub fn token(self) -> &'static str {
        match self {
            Glyph::Keyboard => "kbd",
            Glyph::Wifi => "wifi",
            Glyph::Bluetooth => "bt",
            Glyph::Battery => "bat",
            Glyph::Mouse => "pad",
            Glyph::Crop => "crop",
            Glyph::Qr => "qr",
            Glyph::Calculator => "=",
            _ => self.name(),
        }
    }

    /// The stable, lower-case name: what `--glyphs` prints beside a codepoint
    /// and what a report calls a hole.
    pub fn name(self) -> &'static str {
        match self {
            Glyph::Folder => "folder",
            Glyph::File => "file",
            Glyph::Document => "document",
            Glyph::Pdf => "pdf",
            Glyph::Html => "html",
            Glyph::Archive => "archive",
            Glyph::Executable => "executable",
            Glyph::Hidden => "hidden",
            Glyph::BrokenLink => "broken_link",
            Glyph::Image => "image",
            Glyph::Video => "video",
            Glyph::Music => "music",
            Glyph::Library => "library",
            Glyph::Home => "home",
            Glyph::Desktop => "desktop",
            Glyph::Terminal => "terminal",
            Glyph::Window => "window",
            Glyph::Disk => "disk",
            Glyph::Gear => "gear",
            Glyph::Book => "book",
            Glyph::People => "people",
            Glyph::Globe => "globe",
            Glyph::Link => "link",
            Glyph::Volume => "volume",
            Glyph::Bell => "bell",
            Glyph::BellOff => "bell_off",
            Glyph::Clock => "clock",
            Glyph::Calendar => "calendar",
            Glyph::History => "history",
            Glyph::Trash => "trash",
            Glyph::Bug => "bug",
            Glyph::Mic => "mic",
            Glyph::Sliders => "sliders",
            Glyph::Lock => "lock",
            Glyph::Unlock => "unlock",
            Glyph::Eye => "eye",
            Glyph::Moon => "moon",
            Glyph::Brightness => "brightness",
            Glyph::Keyboard => "keyboard",
            Glyph::Wifi => "wifi",
            Glyph::Bluetooth => "bluetooth",
            Glyph::Battery => "battery",
            Glyph::Mouse => "mouse",
            Glyph::Network => "network",
            Glyph::Server => "server",
            Glyph::Database => "database",
            Glyph::Chip => "chip",
            Glyph::Display => "display",
            Glyph::Power => "power",
            Glyph::Logout => "logout",
            Glyph::Reboot => "reboot",
            Glyph::Rotate => "rotate",
            Glyph::Search => "search",
            Glyph::Warning => "warning",
            Glyph::Info => "info",
            Glyph::Check => "check",
            Glyph::Undo => "undo",
            Glyph::Save => "save",
            Glyph::Sync => "sync",
            Glyph::Download => "download",
            Glyph::Capture => "capture",
            Glyph::Camera => "camera",
            Glyph::Crop => "crop",
            Glyph::Qr => "qr",
            Glyph::Clipboard => "clipboard",
            Glyph::Smile => "smile",
            Glyph::Filter => "filter",
            Glyph::Tag => "tag",
            Glyph::Key => "key",
            Glyph::Map => "map",
            Glyph::Theme => "theme",
            Glyph::Brush => "brush",
            Glyph::Box => "box",
            Glyph::Calculator => "calculator",
            Glyph::Play => "play",
            Glyph::Pause => "pause",
            Glyph::Stop => "stop",
            Glyph::Next => "next",
            Glyph::Prev => "prev",
            Glyph::ArrowUp => "arrow_up",
            Glyph::ArrowDown => "arrow_down",
            Glyph::ArrowLeft => "arrow_left",
            Glyph::ArrowRight => "arrow_right",
            Glyph::Target => "target",
            Glyph::Star => "star",
            Glyph::Heart => "heart",
            Glyph::Pin => "pin",
            Glyph::Lightbulb => "lightbulb",
            Glyph::Cloud => "cloud",
            Glyph::Code => "code",
            Glyph::Agent => "agent",
            Glyph::Sparkle => "sparkle",
            Glyph::Git => "git",
            Glyph::Github => "github",
            Glyph::Docker => "docker",
            Glyph::Rust => "rust",
            Glyph::Python => "python",
            Glyph::Node => "node",
            Glyph::Linux => "linux",
            Glyph::Debian => "debian",
            Glyph::Shield => "shield",
        }
    }

    fn draw(self) -> Option<&'static str> {
        table::TABLE
            .iter()
            .find(|entry| entry.glyph == self)
            .and_then(|entry| entry.draw)
    }
}

/// Say once, on stderr, that some names draw text instead of a picture.
///
/// Only with `LAZYOS_GLYPH_DEBUG` set: a TUI that writes to the terminal it is
/// drawing on corrupts itself, and the fallback is a documented state, not an
/// error.  The machine-readable answer (`lazyos --glyph-font`) is what
/// `check-system.sh` reads.
fn note_text_fallback(glyph: Glyph) {
    use std::sync::atomic::{AtomicBool, Ordering};
    static ONCE: AtomicBool = AtomicBool::new(false);
    if std::env::var_os("LAZYOS_GLYPH_DEBUG").is_none() || ONCE.swap(true, Ordering::Relaxed) {
        return;
    }
    eprintln!(
        "lazyos: {} of {} alphabet names draw text (LAZYOS_GLYPH_DEBUG); first {} -> {}",
        table::FALLBACK,
        VARIANTS.len(),
        glyph.name(),
        glyph.token()
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The same pin `check-system.sh` compares with the installed font.
    const PIN: &str = include_str!("glyphs/font.pin");

    /// The generated source itself, to prove it stayed ASCII.
    const SOURCE: &str = include_str!("glyphs/table.rs");

    fn pin(key: &str) -> &'static str {
        PIN.lines()
            .find_map(|line| line.strip_prefix(key)?.strip_prefix('='))
            .unwrap_or_default()
    }

    #[test]
    fn every_variant_has_an_entry_and_the_table_adds_nothing() {
        assert_eq!(
            VARIANTS.len(),
            table::TABLE.len(),
            "the enum and scripts/make-glyphs.sh disagree"
        );
        for glyph in VARIANTS {
            assert!(
                table::TABLE.iter().any(|entry| entry.glyph == *glyph),
                "{glyph:?} has no table entry: regenerate with scripts/make-glyphs.sh"
            );
        }
        let mut sorted = VARIANTS.to_vec();
        sorted.sort();
        sorted.dedup();
        assert_eq!(sorted.len(), VARIANTS.len(), "a variant is listed twice");
    }

    #[test]
    fn every_variant_draws_a_codepoint_or_a_text_token() {
        for glyph in VARIANTS {
            let cell = glyph.cell();
            assert!(!cell.is_empty(), "{glyph:?} draws nothing at all");
            match glyph.codepoint() {
                Some(ch) => assert_eq!(cell, ch.to_string(), "{glyph:?} (one cell)"),
                None => assert_eq!(cell, glyph.token(), "{glyph:?} (its token)"),
            }
        }
    }

    #[test]
    fn no_two_names_draw_the_same_codepoint() {
        let mut seen = std::collections::BTreeMap::new();
        for glyph in VARIANTS {
            let Some(ch) = glyph.codepoint() else {
                continue;
            };
            if let Some(other) = seen.insert(ch, *glyph) {
                panic!("{ch:?} is both {other:?} and {glyph:?}");
            }
        }
    }

    #[test]
    fn no_codepoint_comes_from_plane_fifteen() {
        // U+F0000.. is Material Design Icons' plane, the one `fonts-nerd-symbols`
        // strips: a name chosen from it would draw a box.
        for glyph in VARIANTS {
            if let Some(ch) = glyph.codepoint() {
                let cp = ch as u32;
                assert!(
                    !(0xF0000..=0xFFFFD).contains(&cp),
                    "{glyph:?} is U+{cp:X}, a plane-15 codepoint the symbols package does not carry"
                );
            }
        }
    }

    #[test]
    fn the_fallback_is_ascii_text_not_a_box() {
        for glyph in VARIANTS {
            if glyph.codepoint().is_some() {
                continue;
            }
            let token = glyph.token();
            assert!(
                token.chars().all(|ch| ch.is_ascii_graphic()),
                "{glyph:?} falls back to {token:?}, which is not text"
            );
            assert!(
                token.chars().count() <= 4,
                "{glyph:?} falls back to {token:?}, too wide for the glyph column"
            );
        }
    }

    #[test]
    fn the_generated_table_is_ascii() {
        assert!(
            SOURCE.is_ascii(),
            "table.rs carries a literal private-use character: regenerate it"
        );
    }

    #[test]
    fn the_pin_matches_the_table() {
        assert_eq!(pin("family"), table::FONT_FAMILY);
        assert_eq!(pin("sha256"), table::FONT_SHA256);
        assert_eq!(pin("file"), table::FONT_FILE);
        assert_eq!(pin("nerd"), table::NERD_VERSION);
        assert_eq!(pin("drawn"), table::DRAWN.to_string());
        assert_eq!(pin("fallback"), table::FALLBACK.to_string());
        assert_eq!(
            table::DRAWN + table::FALLBACK,
            table::TABLE.len(),
            "the pin counts what the table no longer holds"
        );
        assert_eq!(
            table::DRAWN,
            VARIANTS.iter().filter(|g| g.codepoint().is_some()).count()
        );
        assert_eq!(
            table::FALLBACK,
            VARIANTS.iter().filter(|g| g.codepoint().is_none()).count()
        );
    }

    #[test]
    fn names_are_unique_and_are_the_lower_case_form() {
        let mut seen = std::collections::BTreeSet::new();
        for glyph in VARIANTS {
            let name = glyph.name();
            assert!(!name.is_empty(), "{glyph:?} has no name");
            assert!(
                name.chars()
                    .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '_'),
                "{glyph:?} is named {name:?}"
            );
            assert!(seen.insert(name), "two names are both {name:?}");
        }
    }
}
