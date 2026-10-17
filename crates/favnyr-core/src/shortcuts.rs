//! **Configurable** keyboard shortcuts.
//!
//! The core only handles data: a catalogue of bindable actions (`ACTIONS`),
//! parsing/serialization of a combination (`Chord`), and resolution of the
//! effective map (defaults + user overrides) with conflict detection. The
//! GUI supplies the key name **already normalized** (special keys are
//! mapped via Slint's `Key.*` constants), executes the actions, and displays
//! the settings section. No network dependency, 100% local.

use std::collections::BTreeMap;

/// Action category — used for grouping in settings.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActionGroup {
    Navigation,
    Selection,
    TabsViews,
    FileOps,
    ViewOptions,
    App,
}

impl ActionGroup {
    /// i18n key suffix of the group label (resolved on the GUI side).
    pub fn code(self) -> &'static str {
        match self {
            ActionGroup::Navigation => "navigation",
            ActionGroup::Selection => "selection",
            ActionGroup::TabsViews => "tabs_views",
            ActionGroup::FileOps => "file_ops",
            ActionGroup::ViewOptions => "view_options",
            ActionGroup::App => "app",
        }
    }
}

/// Static definition of a bindable action. `id` serves both as the config
/// key (overrides) and as the i18n key suffix of the displayed label.
#[derive(Debug, Clone, Copy)]
pub struct ActionDef {
    pub id: &'static str,
    pub group: ActionGroup,
    /// Default combination (canonical chord). `""` = unassigned by default.
    pub default: &'static str,
}

/// Ordered catalogue of actions (order = display order in settings).
pub const ACTIONS: &[ActionDef] = &[
    // ----- Navigation -----
    ActionDef {
        id: "cursor-up",
        group: ActionGroup::Navigation,
        default: "ArrowUp",
    },
    ActionDef {
        id: "cursor-down",
        group: ActionGroup::Navigation,
        default: "ArrowDown",
    },
    ActionDef {
        id: "cursor-first",
        group: ActionGroup::Navigation,
        default: "Home",
    },
    ActionDef {
        id: "cursor-left",
        group: ActionGroup::Navigation,
        default: "ArrowLeft",
    },
    ActionDef {
        id: "cursor-right",
        group: ActionGroup::Navigation,
        default: "ArrowRight",
    },
    ActionDef {
        id: "cursor-last",
        group: ActionGroup::Navigation,
        default: "End",
    },
    ActionDef {
        id: "nav-back",
        group: ActionGroup::Navigation,
        default: "Alt+ArrowLeft",
    },
    ActionDef {
        id: "nav-forward",
        group: ActionGroup::Navigation,
        default: "Alt+ArrowRight",
    },
    ActionDef {
        id: "nav-parent",
        group: ActionGroup::Navigation,
        default: "Alt+ArrowUp",
    },
    ActionDef {
        id: "nav-home",
        group: ActionGroup::Navigation,
        default: "Alt+Home",
    },
    ActionDef {
        id: "refresh",
        group: ActionGroup::Navigation,
        default: "F5",
    },
    ActionDef {
        id: "focus-address",
        group: ActionGroup::Navigation,
        default: "Ctrl+L",
    },
    // ----- Selection -----
    ActionDef {
        id: "select-all",
        group: ActionGroup::Selection,
        default: "Ctrl+A",
    },
    ActionDef {
        id: "extend-up",
        group: ActionGroup::Selection,
        default: "Shift+ArrowUp",
    },
    ActionDef {
        id: "extend-down",
        group: ActionGroup::Selection,
        default: "Shift+ArrowDown",
    },
    // ----- Tabs & views -----
    ActionDef {
        id: "extend-left",
        group: ActionGroup::Selection,
        default: "Shift+ArrowLeft",
    },
    ActionDef {
        id: "extend-right",
        group: ActionGroup::Selection,
        default: "Shift+ArrowRight",
    },
    ActionDef {
        id: "tab-new",
        group: ActionGroup::TabsViews,
        default: "Ctrl+T",
    },
    ActionDef {
        id: "tab-close",
        group: ActionGroup::TabsViews,
        default: "Ctrl+W",
    },
    ActionDef {
        id: "tab-reopen-closed",
        group: ActionGroup::TabsViews,
        default: "Ctrl+Shift+T",
    },
    ActionDef {
        id: "tab-next",
        group: ActionGroup::TabsViews,
        default: "Ctrl+Tab",
    },
    ActionDef {
        id: "tab-prev",
        group: ActionGroup::TabsViews,
        default: "Ctrl+Shift+Tab",
    },
    ActionDef {
        id: "view-next",
        group: ActionGroup::TabsViews,
        default: "Tab",
    },
    ActionDef {
        id: "view-prev",
        group: ActionGroup::TabsViews,
        default: "Shift+Tab",
    },
    ActionDef {
        id: "split-side",
        group: ActionGroup::TabsViews,
        default: "Ctrl+Backslash",
    },
    ActionDef {
        id: "split-stack",
        group: ActionGroup::TabsViews,
        default: "Ctrl+Shift+Backslash",
    },
    // Shipped unassigned on purpose. The mnemonic sequel to the two splits
    // would be Ctrl+Alt+Backslash, but AltGr IS Ctrl+Alt on Windows and the
    // backslash itself is an AltGr combination on several of the keyboard
    // layouts this ships for. Better no default than one that fights the
    // keyboard; the action is listed, and a chord can be bound to it.
    ActionDef {
        id: "equalize-views",
        group: ActionGroup::TabsViews,
        default: "",
    },
    // ----- File operations -----
    ActionDef {
        id: "open",
        group: ActionGroup::FileOps,
        default: "Enter",
    },
    ActionDef {
        id: "copy",
        group: ActionGroup::FileOps,
        default: "Ctrl+C",
    },
    ActionDef {
        id: "cut",
        group: ActionGroup::FileOps,
        default: "Ctrl+X",
    },
    ActionDef {
        id: "paste",
        group: ActionGroup::FileOps,
        default: "Ctrl+V",
    },
    ActionDef {
        id: "rename",
        group: ActionGroup::FileOps,
        default: "F2",
    },
    ActionDef {
        id: "delete",
        group: ActionGroup::FileOps,
        default: "Delete",
    },
    ActionDef {
        id: "delete-permanent",
        group: ActionGroup::FileOps,
        default: "Shift+Delete",
    },
    ActionDef {
        id: "restore-trash",
        group: ActionGroup::FileOps,
        default: "Ctrl+Z",
    },
    ActionDef {
        id: "new-folder",
        group: ActionGroup::FileOps,
        default: "Ctrl+Shift+N",
    },
    ActionDef {
        id: "new-file",
        group: ActionGroup::FileOps,
        default: "Ctrl+Alt+N",
    },
    ActionDef {
        id: "terminal",
        group: ActionGroup::FileOps,
        // Ctrl+Alt+T is the desktop's own "open a terminal" binding on GNOME
        // and on most KDE setups. The compositor takes the combination before
        // any window is offered it, so the application is never asked and the
        // terminal that appears is the desktop's, started in the home folder.
        // Shift+F4 is free, and is what a file manager usually carries.
        // Windows has no such grab and keeps the familiar combination.
        default: if cfg!(windows) {
            "Ctrl+Alt+T"
        } else {
            "Shift+F4"
        },
    },
    ActionDef {
        id: "properties",
        group: ActionGroup::FileOps,
        default: "Alt+Enter",
    },
    // ----- View options -----
    ActionDef {
        id: "toggle-hidden",
        group: ActionGroup::ViewOptions,
        default: "Ctrl+H",
    },
    ActionDef {
        id: "cycle-group",
        group: ActionGroup::ViewOptions,
        default: "Ctrl+G",
    },
    ActionDef {
        id: "toggle-view-mode",
        group: ActionGroup::ViewOptions,
        default: "Ctrl+P",
    },
    // ----- Application -----
    ActionDef {
        id: "save-workspace",
        group: ActionGroup::App,
        default: "Ctrl+S",
    },
    ActionDef {
        id: "open-settings",
        group: ActionGroup::App,
        default: "Ctrl+Comma",
    },
    ActionDef {
        id: "open-workspaces",
        group: ActionGroup::App,
        default: "Ctrl+Q",
    },
    // One action per keyboard slot. The slot belongs to the workspace, not to
    // the shortcut, so these nine stay fixed while what they load changes with
    // the slots the user hands out.
    ActionDef {
        id: "load-workspace-1",
        group: ActionGroup::App,
        default: "Ctrl+1",
    },
    ActionDef {
        id: "load-workspace-2",
        group: ActionGroup::App,
        default: "Ctrl+2",
    },
    ActionDef {
        id: "load-workspace-3",
        group: ActionGroup::App,
        default: "Ctrl+3",
    },
    ActionDef {
        id: "load-workspace-4",
        group: ActionGroup::App,
        default: "Ctrl+4",
    },
    ActionDef {
        id: "load-workspace-5",
        group: ActionGroup::App,
        default: "Ctrl+5",
    },
    ActionDef {
        id: "load-workspace-6",
        group: ActionGroup::App,
        default: "Ctrl+6",
    },
    ActionDef {
        id: "load-workspace-7",
        group: ActionGroup::App,
        default: "Ctrl+7",
    },
    ActionDef {
        id: "load-workspace-8",
        group: ActionGroup::App,
        default: "Ctrl+8",
    },
    ActionDef {
        id: "load-workspace-9",
        group: ActionGroup::App,
        default: "Ctrl+9",
    },
];

/// Normalized combination. Structural equality means "same shortcut".
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Chord {
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
    /// Canonical key name (e.g. `"A"`, `"ArrowUp"`, `"F2"`, `"Backslash"`).
    pub key: String,
}

impl Chord {
    /// From a keyboard event. `key` must already be a normalized name for
    /// special keys (supplied by the GUI via the `Key.*` constants).
    pub fn from_event(key: &str, ctrl: bool, alt: bool, shift: bool) -> Option<Chord> {
        Some(Chord {
            ctrl,
            alt,
            shift,
            key: normalize_key(key)?,
        })
    }

    /// Parses a text form like "Ctrl+Shift+N". Returns `None` if there's no key.
    pub fn parse(s: &str) -> Option<Chord> {
        let (mut ctrl, mut alt, mut shift) = (false, false, false);
        let mut key = None;
        for part in s.split('+') {
            let p = part.trim();
            if p.is_empty() {
                continue;
            }
            match p.to_ascii_lowercase().as_str() {
                "ctrl" | "control" => ctrl = true,
                "alt" => alt = true,
                "shift" => shift = true,
                _ => key = Some(normalize_key(p)?),
            }
        }
        key.map(|key| Chord {
            ctrl,
            alt,
            shift,
            key,
        })
    }

    /// Canonical form "Ctrl+Alt+Shift+key" (fixed modifier order).
    pub fn serialize(&self) -> String {
        let mut s = String::new();
        if self.ctrl {
            s.push_str("Ctrl+");
        }
        if self.alt {
            s.push_str("Alt+");
        }
        if self.shift {
            s.push_str("Shift+");
        }
        s.push_str(&self.key);
        s
    }
}

/// Translates an ASCII letter or Cyrillic character into its canonical uppercase Latin key ('A'..='Z').
///
/// Fully cross-platform, deterministic, and safe (no OS FFI calls).
/// Covers standard QWERTY Cyrillic keyboard layouts (Russian, Ukrainian `і`, etc.).
/// Returns `None` if the character does not correspond to an A..Z physical key.
pub fn normalize_char(ch: char) -> Option<char> {
    if ch.is_ascii_alphabetic() {
        return Some(ch.to_ascii_uppercase());
    }
    cyrillic_fallback(ch)
}

/// Normalizes a key name to its canonical form. Single letter → uppercase;
/// special names are case-insensitive; unknown → returned as-is (robustness).
fn normalize_key(k: &str) -> Option<String> {
    let k = k.trim();
    if k.is_empty() {
        return None;
    }
    if k.chars().count() == 1 {
        let c = k.chars().next().unwrap();
        if let Some(canon) = normalize_char(c) {
            return Some(canon.to_string());
        }
        return Some(k.to_string()); // digit / literal punctuation
    }
    let lower = k.to_ascii_lowercase();
    // F1..F24
    if let Some(rest) = lower.strip_prefix('f')
        && let Ok(n) = rest.parse::<u8>()
        && (1..=24).contains(&n)
    {
        return Some(format!("F{n}"));
    }
    let canon = match lower.as_str() {
        "arrowup" | "up" => "ArrowUp",
        "arrowdown" | "down" => "ArrowDown",
        "arrowleft" | "left" => "ArrowLeft",
        "arrowright" | "right" => "ArrowRight",
        "tab" => "Tab",
        "enter" | "return" => "Enter",
        "backspace" => "Backspace",
        "delete" | "del" => "Delete",
        "space" => "Space",
        "home" => "Home",
        "end" => "End",
        "pageup" | "pgup" => "PageUp",
        "pagedown" | "pgdn" => "PageDown",
        "insert" | "ins" => "Insert",
        "backslash" => "Backslash",
        "comma" => "Comma",
        _ => return Some(k.to_string()),
    };
    Some(canon.to_string())
}

/// Standard Cyrillic QWERTY layout mapping for Windows, Linux, and macOS.
fn cyrillic_fallback(ch: char) -> Option<char> {
    match ch {
        'й' | 'Й' => Some('Q'),
        'ц' | 'Ц' => Some('W'),
        'у' | 'У' => Some('E'),
        'к' | 'К' => Some('R'),
        'е' | 'Е' => Some('T'),
        'н' | 'Н' => Some('Y'),
        'г' | 'Г' => Some('U'),
        'ш' | 'Ш' => Some('I'),
        'щ' | 'Щ' => Some('O'),
        'з' | 'З' => Some('P'),
        'ф' | 'Ф' => Some('A'),
        'ы' | 'Ы' => Some('S'),
        'в' | 'В' => Some('D'),
        'а' | 'А' => Some('F'),
        'п' | 'П' => Some('G'),
        'р' | 'Р' => Some('H'),
        'о' | 'О' => Some('J'),
        'л' | 'Л' => Some('K'),
        'д' | 'Д' => Some('L'),
        'я' | 'Я' => Some('Z'),
        'ч' | 'Ч' => Some('X'),
        'с' | 'С' => Some('C'),
        'м' | 'М' => Some('V'),
        'и' | 'И' => Some('B'),
        'т' | 'Т' => Some('N'),
        'ь' | 'Ь' => Some('M'),
        'і' | 'І' => Some('S'),
        _ => None,
    }
}

/// Effective map: action_id → serialized chord (override, else default; `""`
/// = unassigned). Built from the config overlays.
pub struct Keymap {
    effective: BTreeMap<String, String>,
}

impl Keymap {
    pub fn build(overrides: &BTreeMap<String, String>) -> Self {
        let mut effective = BTreeMap::new();
        for a in ACTIONS {
            let chord = overrides
                .get(a.id)
                .cloned()
                .unwrap_or_else(|| a.default.to_string());
            effective.insert(a.id.to_string(), chord);
        }
        Keymap { effective }
    }

    /// Effective chord of an action (`""` if unassigned).
    pub fn chord_of(&self, action_id: &str) -> &str {
        self.effective
            .get(action_id)
            .map(|s| s.as_str())
            .unwrap_or("")
    }

    /// Action triggered by this combination (first match in catalogue
    /// order), or `None`.
    pub fn action_for(&self, chord: &Chord) -> Option<&'static str> {
        for a in ACTIONS {
            let eff = self.chord_of(a.id);
            if eff.is_empty() {
                continue;
            }
            if Chord::parse(eff).as_ref() == Some(chord) {
                return Some(a.id);
            }
        }
        None
    }

    /// Another action already using `chord` (other than `action_id`) → conflict.
    pub fn conflict(&self, action_id: &str, chord: &str) -> Option<&'static str> {
        let c = Chord::parse(chord)?;
        for a in ACTIONS {
            if a.id == action_id {
                continue;
            }
            if Chord::parse(self.chord_of(a.id)).as_ref() == Some(&c) {
                return Some(a.id);
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_two_actions_share_a_default_combination() {
        // A duplicate default would hand one combination to two actions, and
        // only one of them would ever answer. The catalogue is the place where
        // that is decided, so it is the place where it is checked.
        let mut seen: std::collections::HashMap<&str, &str> = std::collections::HashMap::new();
        for action in ACTIONS {
            if action.default.is_empty() {
                continue;
            }
            if let Some(other) = seen.insert(action.default, action.id) {
                panic!(
                    "{} and {} both default to {}",
                    other, action.id, action.default
                );
            }
        }
        // The nine workspace slots are there, each on its own digit.
        for slot in 1..=MAX_WORKSPACE_SLOT_ACTIONS {
            let id = format!("load-workspace-{slot}");
            let def = ACTIONS
                .iter()
                .find(|a| a.id == id)
                .unwrap_or_else(|| panic!("missing action {id}"));
            assert_eq!(def.default, format!("Ctrl+{slot}"));
        }
    }

    /// Mirrors the core's slot ceiling without depending on the workspace
    /// module, which the catalogue knows nothing about.
    const MAX_WORKSPACE_SLOT_ACTIONS: u8 = 9;

    #[test]
    fn parse_serialize_round_trip() {
        for s in [
            "Ctrl+C",
            "Ctrl+Shift+N",
            "Alt+ArrowUp",
            "F2",
            "Enter",
            "Ctrl+Backslash",
        ] {
            let c = Chord::parse(s).unwrap();
            assert_eq!(c.serialize(), s, "round-trip {s}");
        }
    }

    #[test]
    fn parse_is_order_and_case_insensitive() {
        let a = Chord::parse("shift+CTRL+n").unwrap();
        assert_eq!(a.serialize(), "Ctrl+Shift+N");
        assert!(a.ctrl && a.shift && !a.alt && a.key == "N");
    }

    #[test]
    fn lowercase_letter_normalizes_to_upper() {
        assert_eq!(Chord::parse("Ctrl+c").unwrap().serialize(), "Ctrl+C");
        // Key from an event (a + shift) → "A" + shift.
        let ev = Chord::from_event("a", false, false, true).unwrap();
        assert_eq!(ev.serialize(), "Shift+A");
    }

    #[test]
    fn default_keymap_resolves_known_chords() {
        let km = Keymap::build(&BTreeMap::new());
        assert_eq!(
            km.action_for(&Chord::parse("Ctrl+C").unwrap()),
            Some("copy")
        );
        assert_eq!(
            km.action_for(&Chord::parse("Alt+ArrowUp").unwrap()),
            Some("nav-parent")
        );
        assert_eq!(
            km.action_for(&Chord::parse("ArrowDown").unwrap()),
            Some("cursor-down")
        );
        for (chord, action) in [
            ("Tab", "view-next"),
            ("Shift+Tab", "view-prev"),
            ("ArrowLeft", "cursor-left"),
            ("ArrowRight", "cursor-right"),
            ("Shift+ArrowLeft", "extend-left"),
            ("Shift+ArrowRight", "extend-right"),
            ("Alt+ArrowLeft", "nav-back"),
            ("Alt+ArrowRight", "nav-forward"),
        ] {
            assert_eq!(km.action_for(&Chord::parse(chord).unwrap()), Some(action));
        }
        assert_eq!(
            km.action_for(&Chord::parse("Ctrl+Z").unwrap()),
            Some("restore-trash")
        );
    }

    #[test]
    fn override_takes_precedence_and_frees_default() {
        let mut ov = BTreeMap::new();
        ov.insert("copy".to_string(), "Ctrl+Insert".to_string());
        let km = Keymap::build(&ov);
        assert_eq!(
            km.action_for(&Chord::parse("Ctrl+Insert").unwrap()),
            Some("copy")
        );
        // Overriding a shortcut frees up its default combination.
        assert_eq!(km.action_for(&Chord::parse("Ctrl+C").unwrap()), None);
        assert_eq!(km.chord_of("copy"), "Ctrl+Insert");
    }

    /// An unassigned action (empty override, as produced by "Remove
    /// shortcut") resolves to `""` and can no longer be triggered, even if
    /// it has a default combination.
    #[test]
    fn unassigned_override_is_empty_and_inert() {
        let mut ov = BTreeMap::new();
        ov.insert("open-workspaces".to_string(), String::new());
        let km = Keymap::build(&ov);
        assert_eq!(km.chord_of("open-workspaces"), "");
        // The default chord no longer triggers anything.
        assert_eq!(km.action_for(&Chord::parse("Ctrl+Q").unwrap()), None);
    }

    #[test]
    fn workspaces_panel_has_ctrl_q_by_default() {
        let km = Keymap::build(&BTreeMap::new());
        assert_eq!(km.chord_of("open-workspaces"), "Ctrl+Q");
        assert_eq!(
            km.action_for(&Chord::parse("Ctrl+Q").unwrap()),
            Some("open-workspaces")
        );
    }

    #[test]
    fn reopen_closed_tab_has_firefox_default() {
        let km = Keymap::build(&BTreeMap::new());
        assert_eq!(km.chord_of("tab-reopen-closed"), "Ctrl+Shift+T");
        assert_eq!(
            km.action_for(&Chord::parse("Ctrl+Shift+T").unwrap()),
            Some("tab-reopen-closed")
        );
    }

    #[test]
    fn save_workspace_has_standard_default() {
        let km = Keymap::build(&BTreeMap::new());
        assert_eq!(km.chord_of("save-workspace"), "Ctrl+S");
        assert_eq!(
            km.action_for(&Chord::parse("Ctrl+S").unwrap()),
            Some("save-workspace")
        );
    }

    #[test]
    fn permanent_delete_has_standard_default() {
        let km = Keymap::build(&BTreeMap::new());
        assert_eq!(km.chord_of("delete-permanent"), "Shift+Delete");
        assert_eq!(
            km.action_for(&Chord::parse("Shift+Delete").unwrap()),
            Some("delete-permanent")
        );
    }

    #[test]
    fn the_terminal_default_avoids_the_desktop_grab_away_from_windows() {
        // GNOME and most KDE setups bind Ctrl+Alt+T themselves, at the
        // compositor, so an application given that default is never offered
        // the key and the action silently does nothing. Measured on KDE: the
        // desktop opens its own terminal and the log records no launch.
        let km = Keymap::build(&BTreeMap::new());
        let chord = km.chord_of("terminal");
        if cfg!(windows) {
            assert_eq!(chord, "Ctrl+Alt+T");
        } else {
            assert_ne!(chord, "Ctrl+Alt+T");
            assert_eq!(chord, "Shift+F4");
        }
        assert_eq!(
            km.action_for(&Chord::parse(chord).unwrap()),
            Some("terminal")
        );
    }

    #[test]
    fn restore_trash_has_standard_default() {
        let km = Keymap::build(&BTreeMap::new());
        assert_eq!(km.chord_of("restore-trash"), "Ctrl+Z");
        assert_eq!(
            km.action_for(&Chord::parse("Ctrl+Z").unwrap()),
            Some("restore-trash")
        );
    }

    #[test]
    fn conflict_detects_existing_binding() {
        let km = Keymap::build(&BTreeMap::new());
        // Ctrl+C is already "copy" → conflict for another action.
        assert_eq!(km.conflict("cut", "Ctrl+C"), Some("copy"));
        // A free combination → no conflict.
        assert_eq!(km.conflict("cut", "Ctrl+Insert"), None);
        // An action's own chord isn't a conflict with itself.
        assert_eq!(km.conflict("copy", "Ctrl+C"), None);
    }

    #[test]
    fn cyrillic_letters_normalize_to_latin() {
        assert_eq!(normalize_char('с'), Some('C'));
        assert_eq!(normalize_char('С'), Some('C'));
        assert_eq!(normalize_char('д'), Some('L'));
        assert_eq!(normalize_char('Д'), Some('L'));
        assert_eq!(normalize_char('т'), Some('N'));
        assert_eq!(normalize_char('Т'), Some('N'));
        assert_eq!(normalize_char('ф'), Some('A'));
        assert_eq!(normalize_char('ч'), Some('X'));
        assert_eq!(normalize_char('м'), Some('V'));
        assert_eq!(normalize_char('я'), Some('Z'));
        assert_eq!(normalize_char('и'), Some('B'));
        assert_eq!(normalize_char('і'), Some('S'));
    }

    #[test]
    fn cyrillic_shortcuts_resolve_in_keymap_and_parse() {
        let km = Keymap::build(&BTreeMap::new());

        // Ctrl+C (ru 'с') -> copy
        let ev_c = Chord::from_event("с", true, false, false).unwrap();
        assert_eq!(ev_c.serialize(), "Ctrl+C");
        assert_eq!(km.action_for(&ev_c), Some("copy"));

        // Ctrl+L (ru 'д') -> focus-address
        let ev_l = Chord::from_event("д", true, false, false).unwrap();
        assert_eq!(ev_l.serialize(), "Ctrl+L");
        assert_eq!(km.action_for(&ev_l), Some("focus-address"));

        // Ctrl+Shift+N (ru 'т' / 'Т') -> new-folder
        let ev_shift_n = Chord::from_event("Т", true, false, true).unwrap();
        assert_eq!(ev_shift_n.serialize(), "Ctrl+Shift+N");
        assert_eq!(km.action_for(&ev_shift_n), Some("new-folder"));

        // Ctrl+Alt+N (ru 'т') -> new-file
        let ev_alt_n = Chord::from_event("т", true, true, false).unwrap();
        assert_eq!(ev_alt_n.serialize(), "Ctrl+Alt+N");
        assert_eq!(km.action_for(&ev_alt_n), Some("new-file"));

        // Parse round-trip from settings recorder
        assert_eq!(Chord::parse("Ctrl+с").unwrap().serialize(), "Ctrl+C");
        assert_eq!(
            Chord::parse("Ctrl+Shift+Т").unwrap().serialize(),
            "Ctrl+Shift+N"
        );
        assert_eq!(
            Chord::parse("Ctrl+Alt+т").unwrap().serialize(),
            "Ctrl+Alt+N"
        );

        // Cyrillic punctuation keys (б, ю, ж) are NOT falsely converted to letters
        assert_eq!(Chord::parse("Ctrl+б").unwrap().serialize(), "Ctrl+б");
    }

    #[test]
    fn non_cyrillic_non_ascii_characters_not_falsely_converted() {
        // German umlauts should not be converted to A..Z
        assert_eq!(normalize_char('ä'), None);
        assert_eq!(normalize_char('ö'), None);
        assert_eq!(normalize_char('ü'), None);
        assert_eq!(normalize_char('ß'), None);

        // Digits remain unchanged
        assert_eq!(Chord::parse("Ctrl+1").unwrap().serialize(), "Ctrl+1");
        assert_eq!(Chord::parse("Ctrl+9").unwrap().serialize(), "Ctrl+9");
    }
}
