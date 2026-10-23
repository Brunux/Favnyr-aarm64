//! The frame budget: a tick at the panel's size must stay under a millisecond.
//!
//! `docs/optimization.md` §13 measured 154 µs for an empty 152x45 frame and
//! 445 µs for a frame that rebuilds 40 formatted rows, so the ceiling is not a
//! guess - it is the number the design was chosen against. A regression here
//! means something started formatting text (or re-solving a layout) per frame
//! instead of copying cached cells.
//!
//! Debug builds are not optimised, so the ceiling is relaxed for them and the
//! release run is the real one: `cargo test --release -p lazyos-ui` on the
//! device is the measurement, `cargo test` is the tripwire.

use std::time::{Duration, Instant};

use lazyos_ui::{App, BodyCtx, Design, Header, KeyHint, ListState, Row, Screen};
use ratatui::Terminal;
use ratatui::backend::TestBackend;

struct Rows {
    list: ListState,
}

impl Screen for Rows {
    fn draw(&mut self, ctx: &mut BodyCtx) {
        self.list.render(ctx.area, ctx.buf, ctx.design, true);
    }
}

#[test]
fn a_tick_of_a_full_list_stays_under_the_budget() {
    const TICKS: u32 = 1_000;
    let mut list = ListState::new();
    list.set_rows(
        (0..40)
            .map(|i| {
                Row::new(format!("row {i:02}"))
                    .glyph("\u{f0ac}")
                    .detail("a detail worth reading")
                    .value("Alt+Ctrl+F")
            })
            .collect(),
    );
    list.set_cursor(20);
    let mut screen = Rows { list };
    let mut app = App::new(
        Header::new("LazyOS")
            .subtitle("the frame budget")
            .status("40 rows"),
    )
    .design(Design::solarized().with_reduced_motion(true))
    .keybar(vec![KeyHint::new("Esc", "close")])
    .screen(&mut screen);
    let mut terminal = Terminal::new(TestBackend::new(152, 45)).unwrap();

    // Warm up: the first draws allocate the buffers and settle the allocator.
    for _ in 0..50 {
        terminal.draw(|frame| app.render(frame)).unwrap();
    }
    let started = Instant::now();
    for _ in 0..TICKS {
        terminal.draw(|frame| app.render(frame)).unwrap();
    }
    let each = started.elapsed() / TICKS;

    let budget = if cfg!(debug_assertions) {
        // Unoptimised: the release number below is the one on the device.
        Duration::from_millis(10)
    } else {
        Duration::from_millis(1)
    };
    assert!(
        each < budget,
        "a 152x45 tick took {each:?}, over the {budget:?} budget \
         (docs/optimization.md §13: 154 µs empty, 445 µs when 40 rows are rebuilt)"
    );
    eprintln!("152x45 tick: {each:?} over {TICKS} frames");
}
