//! The list filter: case does not matter, and a keystroke stays cheap.
//!
//! Two facts are pinned here, and both were chosen against a measurement
//! (`docs/optimization.md` §13). **Case-insensitivity is a contract**, not a
//! nicety: a file browser whose `readme` does not find `README.md` is a broken
//! one, and accented text behaves the same way (`Año` finds `año`).
//!
//! **A keystroke is not a frame.** Filtering happens on a state change, so it
//! may allocate - but not once per row: the first version lowercased the
//! needle inside `Row::matches`, i.e. one `String` per row per keystroke, and
//! the file list is thousands of rows. The allocation test below is what stops
//! that from coming back; the timing test is the other half.

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use lazyos_ui::{ListState, Row};

static ALLOCATIONS: AtomicUsize = AtomicUsize::new(0);

/// The allocator counts what the whole binary allocates, so the tests that
/// read it must not run beside the ones that build rows (`cargo test` runs a
/// file's tests in parallel threads by default).
static SERIAL: std::sync::Mutex<()> = std::sync::Mutex::new(());

struct Counting;

unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        unsafe { System.dealloc(pointer, layout) }
    }

    unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        unsafe { System.realloc(pointer, layout, new_size) }
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        unsafe { System.alloc_zeroed(layout) }
    }
}

#[global_allocator]
static GLOBAL: Counting = Counting;

/// A home directory worth of rows: a name, and the dim metadata beside it.
fn rows(count: usize) -> Vec<Row> {
    (0..count)
        .map(|i| {
            Row::new(format!("report-{i:05}-Quarterly.txt"))
                .detail(format!("{} kB · 2026-10-{:02}", 4 + i % 900, 1 + i % 28))
                .value("Alt+Ctrl+F")
        })
        .collect()
}

fn list(count: usize) -> ListState {
    let mut list = ListState::new();
    list.set_rows(rows(count));
    list
}

#[test]
fn the_filter_is_case_insensitive() {
    let _serial = SERIAL.lock().unwrap_or_else(|poison| poison.into_inner());
    let mut list = ListState::new();
    list.set_rows(vec![
        Row::new("README.md"),
        Row::new("readme.txt"),
        Row::new("Readme"),
        Row::new("notes.md"),
        Row::new("Año nuevo.md"),
        Row::new("AÑO viejo.md"),
        Row::new("ünïcode.txt"),
        Row::new("ÜNÏCODE.txt"),
    ]);

    for needle in ["readme", "README", "ReAdMe"] {
        list.set_filter(needle);
        assert_eq!(
            list.visible_len(),
            3,
            "the filter {needle:?} must match every spelling of README"
        );
    }

    // Accents fold the Unicode way, not only the ASCII way.
    for needle in ["año", "AÑO", "Año"] {
        list.set_filter(needle);
        assert_eq!(
            list.visible_len(),
            2,
            "the filter {needle:?} must match both spellings of año"
        );
    }
    for needle in ["ünï", "ÜNÏ"] {
        list.set_filter(needle);
        assert_eq!(list.visible_len(), 2, "the filter {needle:?} must fold");
    }

    // The filter searches the detail and the value too, the same way.
    let mut list = ListState::new();
    list.set_rows(vec![
        Row::new("first").detail("Quarterly REPORT"),
        Row::new("second").value("Alt+Ctrl+F"),
        Row::new("third"),
    ]);
    list.set_filter("quarterly");
    assert_eq!(list.visible_len(), 1);
    list.set_filter("ALT+CTRL+F");
    assert_eq!(list.visible_len(), 1);
    list.set_filter("THIRD");
    assert_eq!(list.visible_len(), 1);
    list.set_filter("");
    assert_eq!(list.visible_len(), 3, "an empty filter shows everything");
}

/// A needle must not cost an allocation per row: the first version folded the
/// needle inside the row test and folded three fields per row on top of it,
/// i.e. ~20,000 allocations for one keystroke over 5,000 rows. Filtering 200
/// rows and 5,000 rows must cost the same handful of allocations, and that is
/// the assertion - an absolute number would pass with a big list and fail with
/// a bigger one.
#[test]
fn filtering_does_not_allocate_per_row() {
    let _serial = SERIAL.lock().unwrap_or_else(|poison| poison.into_inner());

    /// The best of five passes: the harness allocates on its own thread while
    /// a test runs (starting the next one, buffering output), and the counter
    /// is process-wide, so a single pass can be credited with somebody else's
    /// allocation. A per-row allocation happens on *every* pass, so the minimum
    /// is what tells the two apart.
    fn keystroke(counterpart: &mut ListState, needle: &str) -> usize {
        (0..5)
            .map(|_| {
                let before = ALLOCATIONS.load(Ordering::Relaxed);
                counterpart.set_filter(needle);
                ALLOCATIONS.load(Ordering::Relaxed) - before
            })
            .min()
            .expect("five passes")
    }

    let mut small = list(200);
    small.set_filter("rep");
    let small_typing = keystroke(&mut small, "repo");
    let small_backspace = keystroke(&mut small, "rep");

    let mut large = list(5_000);
    large.set_filter("rep");
    let large_typing = keystroke(&mut large, "repo");
    let large_backspace = keystroke(&mut large, "rep");

    assert_eq!(
        (large_typing, large_backspace),
        (small_typing, small_backspace),
        "a keystroke's allocations must not depend on the number of rows \
         (200 rows: {small_typing} typing / {small_backspace} backspace; \
         5,000 rows: {large_typing} typing / {large_backspace} backspace)"
    );
    assert!(
        large_typing <= 2 && large_backspace <= 2,
        "a keystroke over 5,000 rows allocated {large_typing} (typing) and \
         {large_backspace} (backspace) times; the needle and the match vector \
         are the only things allowed to allocate"
    );
}

/// The two costs a person can feel: typing the next character (the needle
/// grows, so only the visible rows are tested) and backspacing into a rebuild
/// (the needle shrinks, so the whole list is scanned again). The assertion is
/// on the **worst** key, because a long needle is the one that makes a row do
/// the full comparison - an average hides it.
#[test]
fn typing_a_large_list_stays_under_the_budget() {
    let _serial = SERIAL.lock().unwrap_or_else(|poison| poison.into_inner());
    const ROWS: usize = 5_000;

    fn slowest(list: &mut ListState, keys: &[&str]) -> Duration {
        let mut worst = Duration::ZERO;
        for key in keys {
            let started = Instant::now();
            list.set_filter(*key);
            worst = worst.max(started.elapsed());
        }
        worst
    }

    /// The best of a few passes: this machine runs the desktop it is testing,
    /// and a loaded core inflates a single timing by 2-3x. The minimum is the
    /// cost of the code; the budget then only has to catch a change of shape.
    fn best_of(list: &mut ListState, keys: &[&str]) -> Duration {
        (0..5)
            .map(|_| slowest(list, keys))
            .min()
            .expect("five passes")
    }

    let mut list = list(ROWS);
    // Warm up: the first filter allocates the match vector and touches memory.
    list.set_filter("r");
    list.set_filter("re");

    let typing = best_of(
        &mut list,
        &["rep", "repo", "repor", "report", "report-0000"],
    );
    let rebuild = best_of(&mut list, &["report-0000", "report", "rep", "re", "r"]);
    eprintln!("5,000 rows, worst key: typing {typing:?}, rebuild {rebuild:?}");

    let (typing_budget, rebuild_budget) = if cfg!(debug_assertions) {
        // Unoptimised; `cargo test --release -p lazyos-ui` is the device number
        // (docs/optimization.md §13: 372 µs, from 1.74 ms).
        (Duration::from_millis(25), Duration::from_millis(25))
    } else {
        (Duration::from_millis(3), Duration::from_millis(3))
    };
    assert!(
        typing < typing_budget,
        "typing over {ROWS} rows took {typing:?} for the worst key, over the {typing_budget:?} budget"
    );
    assert!(
        rebuild < rebuild_budget,
        "backspacing over {ROWS} rows took {rebuild:?} for the worst key, over the {rebuild_budget:?} budget"
    );
}

/// Narrowing is an optimisation with a proof obligation: `"repo"` can only
/// match rows that `"rep"` matched, so the pass may keep the visible set and
/// drop rows from it. A test that only counted matches would not catch a
/// broken proof - the sequence below must land exactly where a fresh filter
/// would.
#[test]
fn narrowing_gives_the_same_answer_as_a_fresh_filter() {
    let _serial = SERIAL.lock().unwrap_or_else(|poison| poison.into_inner());
    let rows = || {
        vec![
            Row::new("report-Quarterly.txt").detail("final"),
            Row::new("report-annual.txt").detail("draft"),
            Row::new("REPORT-notes.md"),
            Row::new("repos.md"),
            Row::new("republic.txt"),
            Row::new("photo.png").detail("report"),
        ]
    };

    fn labels(list: &mut ListState) -> Vec<String> {
        (0..list.visible_len())
            .map(|i| {
                list.set_cursor(i);
                list.selected().unwrap().label.clone()
            })
            .collect()
    }

    let mut incremental = ListState::new();
    incremental.set_rows(rows());
    for needle in ["r", "re", "rep", "repo", "repor", "repo", "rep", "r", ""] {
        incremental.set_filter(needle);
        let mut fresh = ListState::new();
        fresh.set_rows(rows());
        fresh.set_filter(needle);
        assert_eq!(
            labels(&mut incremental),
            labels(&mut fresh),
            "after typing {needle:?} the narrowed set must equal a fresh filter"
        );
    }

    // New rows under a filter that is already applied: the narrowing pass must
    // see them, not the set that belonged to the previous rows.
    let mut list = ListState::new();
    list.set_rows(rows());
    list.set_filter("rep");
    let before = list.visible_len();
    list.set_rows(rows());
    assert_eq!(
        list.visible_len(),
        before,
        "set_rows must re-filter the new rows under the current filter"
    );
}
