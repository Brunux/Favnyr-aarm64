//! Fills the internal properties panel.
//!
//! The core answers what is true of a path; this module turns those facts
//! into the rows the panel shows. Every figure is formatted and every label
//! translated here, so the Slint side only renders a list and never decides
//! what belongs in it. A fact the filesystem does not report produces no row
//! at all, rather than a row with nothing in it.

use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use favnyr_core::checksum;
use favnyr_core::fs as rfs;
use favnyr_core::i18n::Lang;
#[cfg(unix)]
use favnyr_core::properties::{Access, AccessClass};
use favnyr_core::properties::{ItemKind, ItemProperties};
use slint::{ComponentHandle, Model, ModelRc, SharedString, VecModel};

use crate::i18n;
use crate::{MainWindow, PropRow};

/// Builds every row of the general tab, in display order.
///
/// `offset_secs` is the timezone shift already resolved by the caller, the
/// same one the Modified column uses, so a date reads identically in both
/// places.
fn general_rows(p: &ItemProperties, lang: Lang, offset_secs: i64) -> Vec<PropRow> {
    let mut rows = Vec::with_capacity(8);
    let mut push = |key: &str, value: String| {
        rows.push(PropRow {
            label: i18n::tr(lang, key).into(),
            value: value.into(),
        });
    };

    push("prop_row_type", kind_label(p, lang));

    // A link's destination is the first thing someone opening this panel on a
    // link wants, so it sits right under the type.
    if let Some(target) = &p.link_target {
        push("prop_row_target", target.to_string_lossy().into_owned());
    }

    push("prop_row_location", p.parent.to_string_lossy().into_owned());

    // Directories report the size of their own entry, which would be
    // misleading next to a file's. The recursive total is a walk and belongs
    // to the columns that already perform it.
    if p.kind != ItemKind::Directory {
        push(
            "prop_row_size",
            rfs::format_size(p.size, i18n::size_units(lang)),
        );
    }
    if let Some(on_disk) = p.size_on_disk {
        push(
            "prop_row_size_on_disk",
            rfs::format_size(on_disk, i18n::size_units(lang)),
        );
    }

    for (key, stamp) in [
        ("prop_row_created", p.created),
        ("prop_row_modified", p.modified),
        ("prop_row_accessed", p.accessed),
    ] {
        if let Some(unix) = stamp.and_then(unix_seconds) {
            push(key, rfs::format_mtime(unix, offset_secs));
        }
    }

    rows
}

/// What the item is, in one translated word.
///
/// A symlink says what it points at, because "link" alone leaves the reader
/// to open the panel again on the destination. A link whose destination
/// cannot be reached says so: a broken link is worth naming.
fn kind_label(p: &ItemProperties, lang: Lang) -> String {
    let key = match p.kind {
        ItemKind::File => "prop_kind_file",
        ItemKind::Directory => "prop_kind_folder",
        ItemKind::Other => "prop_kind_other",
        ItemKind::Symlink => match p.target_kind {
            Some(ItemKind::Directory) => "prop_kind_link_folder",
            Some(_) => "prop_kind_link_file",
            None => "prop_kind_link_broken",
        },
    };
    i18n::tr(lang, key)
}

/// Seconds since the epoch, or `None` for a stamp that precedes it, which a
/// few filesystems do report and which no formatter here handles.
fn unix_seconds(time: SystemTime) -> Option<i64> {
    time.duration_since(UNIX_EPOCH)
        .ok()
        .map(|d| d.as_secs() as i64)
}

/// Reads `path` and shows the panel on its general tab.
///
/// Returns `false` when the item cannot be read at all, leaving the caller to
/// report it: showing an empty panel would say less than the error does.
pub fn open(
    window: &MainWindow,
    path: &Path,
    lang: Lang,
    offset_secs: i64,
    icon: slint::Image,
) -> bool {
    let Ok(facts) = ItemProperties::read(path) else {
        return false;
    };

    window.set_prop_path(SharedString::from(path.to_string_lossy().as_ref()));
    window.set_prop_name(SharedString::from(facts.name.as_str()));
    window.set_prop_icon(icon);
    window.set_prop_has_permissions(facts.unix.is_some());
    window.set_prop_can_hash(facts.kind != ItemKind::Directory);
    window.set_prop_digest_rows(ModelRc::new(VecModel::<PropRow>::default()));
    window.set_prop_expected(SharedString::new());
    window.set_prop_expected_verdict(-1);
    window.set_prop_hashing(false);
    fill_permissions(window, &facts, lang);
    window.set_prop_general_rows(ModelRc::new(VecModel::from(general_rows(
        &facts,
        lang,
        offset_secs,
    ))));
    // Always land on the first tab: the panel is reused, and reopening it on
    // whichever tab was last seen would surprise.
    window.set_prop_tab(0);
    window.set_prop_open(true);
    true
}

/// The three label lists and the three chosen indices of the permissions tab.
///
/// A class whose bits say something the three usual choices cannot express
/// gets a fourth entry naming that state. Keeping it in the list is what lets
/// the panel show an unusual mode without rounding it to a neighbour, and
/// choosing it applies nothing, which is what the core already guarantees.
#[cfg(unix)]
fn access_choices(access: Access, lang: Lang) -> (Vec<SharedString>, i32) {
    let mut labels: Vec<SharedString> = vec![
        i18n::tr(lang, "prop_access_forbidden").into(),
        i18n::tr(lang, "prop_access_read").into(),
        i18n::tr(lang, "prop_access_read_write").into(),
    ];
    let index = match access {
        Access::Forbidden => 0,
        Access::ReadOnly => 1,
        Access::ReadWrite => 2,
        Access::Custom => {
            labels.push(i18n::tr(lang, "prop_access_custom").into());
            3
        }
    };
    (labels, index)
}

/// Turns a chosen index back into the permission it names.
///
/// The fourth entry only ever describes what was read, so it maps to the
/// state that the core refuses to apply.
#[cfg(unix)]
fn access_from_index(index: i32) -> Access {
    match index {
        0 => Access::Forbidden,
        1 => Access::ReadOnly,
        2 => Access::ReadWrite,
        _ => Access::Custom,
    }
}

/// The class a tab row stands for.
#[cfg(unix)]
pub fn access_class(index: i32) -> AccessClass {
    match index {
        0 => AccessClass::Owner,
        1 => AccessClass::Group,
        _ => AccessClass::Others,
    }
}

/// Fills the permissions tab from `facts`.
///
/// The owner and group names are read here and nowhere else: resolving them
/// opens the system account database, so a panel whose permissions tab is
/// never shown never opens it. An identifier with no matching entry shows as
/// the number, which is also what happens where accounts live in a directory
/// service.
#[cfg(unix)]
fn fill_permissions(window: &MainWindow, facts: &ItemProperties, lang: Lang) {
    let Some(unix) = &facts.unix else { return };

    for (class, set_labels, set_index) in [
        (
            AccessClass::Owner,
            MainWindow::set_prop_access_labels_owner as fn(&MainWindow, ModelRc<SharedString>),
            MainWindow::set_prop_access_owner as fn(&MainWindow, i32),
        ),
        (
            AccessClass::Group,
            MainWindow::set_prop_access_labels_group as fn(&MainWindow, ModelRc<SharedString>),
            MainWindow::set_prop_access_group as fn(&MainWindow, i32),
        ),
        (
            AccessClass::Others,
            MainWindow::set_prop_access_labels_others as fn(&MainWindow, ModelRc<SharedString>),
            MainWindow::set_prop_access_others as fn(&MainWindow, i32),
        ),
    ] {
        let (labels, index) = access_choices(unix.access(class), lang);
        set_labels(window, ModelRc::new(VecModel::from(labels)));
        set_index(window, index);
    }

    window.set_prop_executable(unix.owner_executable());
    window.set_prop_owner(SharedString::from(
        unix.owner_name().unwrap_or_else(|| unix.uid.to_string()),
    ));
    window.set_prop_group(SharedString::from(
        unix.group_name().unwrap_or_else(|| unix.gid.to_string()),
    ));
    // Four octal digits, so the setuid, setgid and sticky bits are visible to
    // whoever reads a mode that way.
    window.set_prop_mode(SharedString::from(format!("{:04o}", unix.mode)));
}

#[cfg(not(unix))]
fn fill_permissions(_window: &MainWindow, _facts: &ItemProperties, _lang: Lang) {}

/// Applies a chosen permission and reports the new state.
///
/// The file is read back rather than assumed: a refused change must leave the
/// panel showing what is actually on disk, not what was asked for.
#[cfg(unix)]
pub fn apply_access(
    window: &MainWindow,
    path: &Path,
    class: AccessClass,
    index: i32,
    lang: Lang,
) -> Result<(), String> {
    favnyr_core::properties::set_access(path, class, access_from_index(index))
        .map_err(|e| e.to_string())?;
    refresh_permissions(window, path, lang);
    Ok(())
}

/// Applies the execute flag, under the same read-back rule.
#[cfg(unix)]
pub fn apply_executable(
    window: &MainWindow,
    path: &Path,
    executable: bool,
    lang: Lang,
) -> Result<(), String> {
    favnyr_core::properties::set_executable(path, executable).map_err(|e| e.to_string())?;
    refresh_permissions(window, path, lang);
    Ok(())
}

/// Repaints the tab after a refused change, so the controls go back to what
/// the file actually carries instead of keeping the choice that was declined.
#[cfg(unix)]
pub fn refresh_after_failure(window: &MainWindow, path: &Path, lang: Lang) {
    refresh_permissions(window, path, lang);
}

/// Re-reads the item and repaints the permissions tab from it.
#[cfg(unix)]
fn refresh_permissions(window: &MainWindow, path: &Path, lang: Lang) {
    if let Ok(facts) = ItemProperties::read(path) {
        fill_permissions(window, &facts, lang);
    }
}

// Digest work in flight, so a second request or a closing panel can stop it.
// One slot is enough: the panel describes one item at a time, and starting a
// computation first cancels whatever the previous one was doing.
thread_local! {
    static HASHING: std::cell::RefCell<Option<Arc<AtomicBool>>> =
        const { std::cell::RefCell::new(None) };
}

/// Stops any computation in flight and forgets it.
pub fn cancel_hashing() {
    HASHING.with(|h| {
        if let Some(flag) = h.borrow_mut().take() {
            flag.store(true, Ordering::Relaxed);
        }
    });
}

/// Reads the item off the UI thread and fills the checksums tab.
///
/// The file is read once for all four algorithms. Progress is reported as a
/// fraction so the bar means something on a large file, and the whole thing
/// can be abandoned at any chunk boundary.
pub fn start_hashing(window: &MainWindow, path: &Path) {
    cancel_hashing();

    let Ok(total) = std::fs::metadata(path).map(|m| m.len()) else {
        return;
    };
    let cancel = Arc::new(AtomicBool::new(false));
    HASHING.with(|h| *h.borrow_mut() = Some(cancel.clone()));

    window.set_prop_hashing(true);
    window.set_prop_hash_progress(0.0);
    window.set_prop_digest_rows(ModelRc::new(VecModel::<PropRow>::default()));
    window.set_prop_expected_verdict(-1);

    let weak = window.as_weak();
    let owned = path.to_path_buf();
    std::thread::spawn(move || {
        let progress_weak = weak.clone();
        let outcome = checksum::hash_file(&owned, &checksum::Algorithm::ALL, &cancel, |done| {
            // An empty file would divide by zero, and has nothing to show
            // anyway: it finishes before the first report.
            let fraction = if total == 0 {
                1.0
            } else {
                done as f32 / total as f32
            };
            let w = progress_weak.clone();
            let _ = slint::invoke_from_event_loop(move || {
                if let Some(w) = w.upgrade() {
                    w.set_prop_hash_progress(fraction);
                }
            });
        });

        let _ = slint::invoke_from_event_loop(move || {
            let Some(w) = weak.upgrade() else { return };
            w.set_prop_hashing(false);
            // A cancelled or failed read leaves the tab as it was, with its
            // Compute button back: half a digest would be a lie.
            let Ok(Some(digests)) = outcome else { return };
            w.set_prop_digest_rows(ModelRc::new(VecModel::from(digest_rows(&digests))));
            HASHING.with(|h| *h.borrow_mut() = None);
        });
    });
}

/// One row per algorithm, the label being the name a download page uses.
fn digest_rows(digests: &[(checksum::Algorithm, String)]) -> Vec<PropRow> {
    digests
        .iter()
        .map(|(algorithm, value)| PropRow {
            label: algorithm.label().into(),
            value: value.as_str().into(),
        })
        .collect()
}

/// Compares what the reader pasted against the computed digests.
///
/// Nothing typed clears the verdict rather than calling it a mismatch: an
/// empty field is not a failed comparison.
pub fn compare_expected(window: &MainWindow, candidate: &str, lang: Lang) {
    window.set_prop_expected(SharedString::from(candidate));
    if candidate.trim().is_empty() {
        window.set_prop_expected_verdict(-1);
        return;
    }
    let digests: Vec<(checksum::Algorithm, String)> = window
        .get_prop_digest_rows()
        .iter()
        .filter_map(|row| {
            checksum::Algorithm::ALL
                .into_iter()
                .find(|a| a.label() == row.label.as_str())
                .map(|a| (a, row.value.to_string()))
        })
        .collect();

    match checksum::matches_any(&digests, candidate) {
        Some(algorithm) => {
            window.set_prop_expected_match(SharedString::from(
                i18n::tr(lang, "prop_expected_matched").replace("{algorithm}", algorithm.label()),
            ));
            window.set_prop_expected_verdict(1);
        }
        None => window.set_prop_expected_verdict(0),
    }
}

/// Clears what the panel held. Called when it closes, so a large path or a
/// long list of rows does not stay alive until the next item is inspected.
pub fn close(window: &MainWindow) {
    // A computation left running would keep reading a file nobody is looking
    // at any more.
    cancel_hashing();
    window.set_prop_hashing(false);
    window.set_prop_digest_rows(ModelRc::new(VecModel::<PropRow>::default()));
    window.set_prop_expected(SharedString::new());
    window.set_prop_expected_verdict(-1);
    window.set_prop_path(SharedString::new());
    window.set_prop_name(SharedString::new());
    window.set_prop_general_rows(ModelRc::new(VecModel::<PropRow>::default()));
}

#[cfg(test)]
mod tests {
    use super::*;

    fn labels(rows: &[PropRow]) -> Vec<String> {
        rows.iter().map(|r| r.label.to_string()).collect()
    }

    fn value_at(rows: &[PropRow], index: usize) -> String {
        rows[index].value.to_string()
    }

    /// A scratch directory of our own, removed on drop.
    struct Scratch(std::path::PathBuf);

    impl Scratch {
        fn new(tag: &str) -> Self {
            let dir =
                std::env::temp_dir().join(format!("favnyr-proprows-{tag}-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&dir);
            std::fs::create_dir_all(&dir).expect("scratch directory");
            Self(dir)
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn a_file_lists_its_type_location_and_size() {
        let scratch = Scratch::new("file");
        let path = scratch.0.join("my_file.txt");
        std::fs::write(&path, b"0123456789").expect("write");

        let facts = ItemProperties::read(&path).expect("read");
        let rows = general_rows(&facts, Lang::En, 0);
        let keys = labels(&rows);

        assert_eq!(keys[0], i18n::tr(Lang::En, "prop_row_type"));
        assert!(keys.contains(&i18n::tr(Lang::En, "prop_row_location")));
        assert!(keys.contains(&i18n::tr(Lang::En, "prop_row_size")));
        // No destination row for something that is not a link.
        assert!(!keys.contains(&i18n::tr(Lang::En, "prop_row_target")));
    }

    #[test]
    fn a_directory_has_no_size_row() {
        let scratch = Scratch::new("dir");
        let path = scratch.0.join("folder01");
        std::fs::create_dir(&path).expect("create dir");

        let facts = ItemProperties::read(&path).expect("read");
        let keys = labels(&general_rows(&facts, Lang::En, 0));
        // The entry's own size would be read as the folder's contents.
        assert!(!keys.contains(&i18n::tr(Lang::En, "prop_row_size")));
    }

    #[test]
    fn the_timezone_offset_shifts_the_dates() {
        let scratch = Scratch::new("tz");
        let path = scratch.0.join("my_file.txt");
        std::fs::write(&path, b"x").expect("write");

        let facts = ItemProperties::read(&path).expect("read");
        let utc = general_rows(&facts, Lang::En, 0);
        let shifted = general_rows(&facts, Lang::En, 3600);

        let modified = i18n::tr(Lang::En, "prop_row_modified");
        let a = utc.iter().position(|r| r.label == modified).expect("row");
        let b = shifted
            .iter()
            .position(|r| r.label == modified)
            .expect("row");
        assert_ne!(value_at(&utc, a), value_at(&shifted, b));
    }

    #[cfg(unix)]
    #[test]
    fn a_link_names_its_destination_and_its_breakage() {
        let scratch = Scratch::new("link");
        let target = scratch.0.join("my_file.txt");
        std::fs::write(&target, b"x").expect("write");
        let good = scratch.0.join("my_link");
        std::os::unix::fs::symlink(&target, &good).expect("symlink");
        let bad = scratch.0.join("my_broken_link");
        std::os::unix::fs::symlink(scratch.0.join("absent"), &bad).expect("symlink");

        let facts = ItemProperties::read(&good).expect("read");
        let rows = general_rows(&facts, Lang::En, 0);
        assert!(labels(&rows).contains(&i18n::tr(Lang::En, "prop_row_target")));
        assert_eq!(
            value_at(&rows, 0),
            i18n::tr(Lang::En, "prop_kind_link_file")
        );

        let facts = ItemProperties::read(&bad).expect("read");
        let rows = general_rows(&facts, Lang::En, 0);
        assert_eq!(
            value_at(&rows, 0),
            i18n::tr(Lang::En, "prop_kind_link_broken")
        );
    }

    #[cfg(unix)]
    #[test]
    fn the_three_usual_states_offer_three_choices() {
        for (access, expected) in [
            (Access::Forbidden, 0),
            (Access::ReadOnly, 1),
            (Access::ReadWrite, 2),
        ] {
            let (labels, index) = access_choices(access, Lang::En);
            assert_eq!(labels.len(), 3, "{access:?}");
            assert_eq!(index, expected, "{access:?}");
        }
    }

    #[cfg(unix)]
    #[test]
    fn an_unusual_mode_gains_a_fourth_choice_that_applies_nothing() {
        let (labels, index) = access_choices(Access::Custom, Lang::En);
        // Shown rather than rounded to a neighbour, which would silently
        // rewrite a permission the owner set on purpose.
        assert_eq!(labels.len(), 4);
        assert_eq!(index, 3);
        assert_eq!(labels[3], i18n::tr(Lang::En, "prop_access_custom"));
        // And choosing it maps back to the state the core refuses to write.
        assert_eq!(access_from_index(3), Access::Custom);
    }

    #[cfg(unix)]
    #[test]
    fn a_chosen_index_names_the_permission_it_shows() {
        for (index, expected) in [
            (0, Access::Forbidden),
            (1, Access::ReadOnly),
            (2, Access::ReadWrite),
        ] {
            assert_eq!(access_from_index(index), expected);
            // The round trip must hold, or a row would move on its own the
            // moment the tab is repainted.
            assert_eq!(access_choices(expected, Lang::En).1, index);
        }
    }

    #[cfg(unix)]
    #[test]
    fn the_three_rows_name_the_three_classes() {
        assert_eq!(access_class(0), AccessClass::Owner);
        assert_eq!(access_class(1), AccessClass::Group);
        assert_eq!(access_class(2), AccessClass::Others);
    }

    #[test]
    fn digest_rows_carry_the_name_a_download_page_uses() {
        let digests = favnyr_core::checksum::hash_bytes(b"abc", &checksum::Algorithm::ALL);
        let rows = digest_rows(&digests);
        let labels: Vec<String> = rows.iter().map(|r| r.label.to_string()).collect();
        assert_eq!(labels, ["MD5", "SHA1", "SHA256", "SHA512"]);
        assert_eq!(rows[0].value.as_str(), "900150983cd24fb0d6963f7d28e17f72");
    }

    #[test]
    fn every_label_used_here_exists_in_the_catalog() {
        // `tr` falls back to the key itself, so a missing translation shows
        // up as a raw identifier in the panel rather than as a failure.
        for key in [
            "prop_hash_compute",
            "prop_hash_cancel_label",
            "prop_hash_hint",
            "prop_hash_folder",
            "prop_expected_label",
            "prop_expected_no_match",
            "prop_expected_matched",
            "prop_perm_owner",
            "prop_perm_group",
            "prop_perm_others",
            "prop_perm_execute",
            "prop_perm_ownership",
            "prop_perm_mode",
            "prop_access_forbidden",
            "prop_access_read",
            "prop_access_read_write",
            "prop_access_custom",
            "prop_perm_failed",
            "prop_row_type",
            "prop_row_target",
            "prop_row_location",
            "prop_row_size",
            "prop_row_size_on_disk",
            "prop_row_created",
            "prop_row_modified",
            "prop_row_accessed",
            "prop_kind_file",
            "prop_kind_folder",
            "prop_kind_other",
            "prop_kind_link_file",
            "prop_kind_link_folder",
            "prop_kind_link_broken",
        ] {
            assert_ne!(i18n::tr(Lang::En, key), key, "{key} is not translated");
        }
    }
}
