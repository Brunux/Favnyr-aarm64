//! The widgets the framework draws with.
//!
//! Spec 001 ships the two states the host can reach without any command behind
//! it - an **empty state** ("this menu arrives in spec 007") and an **error
//! card** (a command failed) - because they are what proves the terminal, the
//! palette and the snapshot harness before a single menu exists. Spec 002 grows
//! this into the design system (`Row`, `List`, `Card`, key bar, motion); nothing
//! here may be duplicated there.

use lazyos_theme::{Palette, Rgb};
use ratatui::Frame;
use ratatui::buffer::Buffer;
use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Padding, Paragraph, Wrap};

/// The theme's colour in the terminal library's type.
pub fn color(c: Rgb) -> Color {
    Color::Rgb(c.r, c.g, c.b)
}

/// A rectangle centred in `area`, `percent_x` wide and `height` rows tall.
///
/// The arithmetic is saturating on purpose: a terminal narrower than the card
/// min width (a popup in a tiny window, an `ssh` session at 20 columns) must
/// draw a smaller card, not panic on an underflow.
pub fn centered(area: Rect, percent_x: u16, height: u16) -> Rect {
    let width = area
        .width
        .saturating_mul(percent_x)
        .saturating_div(100)
        .max(24)
        .min(area.width);
    let height = height.min(area.height);
    Rect {
        x: area.x + area.width.saturating_sub(width) / 2,
        y: area.y + area.height.saturating_sub(height) / 2,
        width,
        height,
    }
}

/// Paint the whole area with the session background and text colour.
///
/// Without this, a window is the terminal's default colours with the session's
/// text on top - the one thing that made a floating menu look foreign.
pub fn backdrop(frame: &mut Frame, area: Rect, palette: &Palette) {
    frame.render_widget(
        Block::default().style(
            Style::default()
                .bg(color(palette.bg))
                .fg(color(palette.text)),
        ),
        area,
    );
}

fn card(palette: &Palette, title: &str, accent: Rgb) -> Block<'static> {
    Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(color(accent)))
        .padding(Padding::new(2, 2, 1, 1))
        .title(Span::styled(
            format!(" {title} "),
            Style::default()
                .fg(color(accent))
                .add_modifier(Modifier::BOLD),
        ))
        .style(
            Style::default()
                .bg(color(palette.bg))
                .fg(color(palette.text)),
        )
}

/// The state of a command that has nothing to show yet.
///
/// `hint` is where the key that closes the window is named: a card with no way
/// out is the failure the session's manifests keep hitting in practice.
pub fn empty_state(frame: &mut Frame, area: Rect, palette: &Palette, title: &str, hint: &str) {
    backdrop(frame, area, palette);
    let rect = centered(area, 62, 9);
    let body = vec![
        Line::from(""),
        Line::from(Span::styled(
            title,
            Style::default()
                .fg(color(palette.accent))
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(hint, Style::default().fg(color(palette.dim)))),
    ];
    let paragraph = Paragraph::new(body)
        .block(card(palette, "LazyOS", palette.accent))
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: true });
    frame.render_widget(paragraph, rect);
}

/// The state of a command that failed: what failed, and the detail a person
/// needs to fix it (the same two lines the shell's `fail()` printed).
pub fn error_card(frame: &mut Frame, area: Rect, palette: &Palette, summary: &str, detail: &str) {
    backdrop(frame, area, palette);
    let rect = centered(area, 76, 11);
    let body = vec![
        Line::from(""),
        Line::from(Span::styled(
            summary,
            Style::default()
                .fg(color(palette.danger))
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(
            detail,
            Style::default().fg(color(palette.text)),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "press any key to close",
            Style::default().fg(color(palette.dim)),
        )),
    ];
    let paragraph = Paragraph::new(body)
        .block(card(
            palette,
            "LazyOS: something went wrong",
            palette.danger,
        ))
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: true });
    frame.render_widget(paragraph, rect);
}

/// A buffer as text, one line per row, for tests that want to read the screen
/// instead of indexing cells.
#[doc(hidden)]
pub fn screen(buf: &Buffer) -> String {
    let mut out = String::with_capacity((buf.area.width as usize + 1) * buf.area.height as usize);
    for y in 0..buf.area.height {
        for x in 0..buf.area.width {
            out.push_str(buf.cell((x, y)).map(|c| c.symbol()).unwrap_or(" "));
        }
        out.push('\n');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    fn draw(width: u16, height: u16, f: impl FnOnce(&mut Frame, Rect, &Palette)) -> String {
        let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
        let palette = Palette::solarized_dark();
        terminal
            .draw(|frame| {
                let area = frame.area();
                f(frame, area, &palette)
            })
            .unwrap();
        screen(terminal.backend().buffer())
    }

    #[test]
    fn empty_state_names_the_title_the_hint_and_the_way_out() {
        let text = draw(80, 24, |frame, area, palette| {
            empty_state(
                frame,
                area,
                palette,
                "the session menu",
                "arrives in spec 007 - Esc closes",
            )
        });
        assert!(text.contains("LazyOS"), "the card carries the brand");
        assert!(text.contains("the session menu"));
        assert!(text.contains("arrives in spec 007"));
        assert!(
            text.contains('╭'),
            "rounded corners are the design language"
        );
    }

    #[test]
    fn error_card_shows_the_summary_and_the_detail() {
        let text = draw(80, 24, |frame, area, palette| {
            error_card(
                frame,
                area,
                palette,
                "the menu does not know that name",
                "try: lazyos menu",
            )
        });
        assert!(text.contains("the menu does not know"));
        assert!(text.contains("try: lazyos menu"));
        assert!(text.contains("press any key"));
    }

    #[test]
    fn the_cards_survive_the_panels_grid_and_a_tiny_terminal() {
        for (w, h) in [(152, 45), (80, 24), (24, 8), (12, 4)] {
            let text = draw(w, h, |frame, area, palette| {
                error_card(frame, area, palette, "failed", "because")
            });
            assert_eq!(text.lines().count(), h as usize);
        }
    }

    #[test]
    fn centered_never_leaves_the_area() {
        let area = Rect::new(0, 0, 152, 45);
        let rect = centered(area, 76, 11);
        assert!(rect.x + rect.width <= area.width);
        assert!(rect.y + rect.height <= area.height);
        assert_eq!(rect.width, 115, "76 % of 152");
        let tiny = centered(Rect::new(0, 0, 10, 3), 76, 11);
        assert_eq!(tiny.width, 10);
        assert_eq!(tiny.height, 3);
    }

    #[test]
    fn the_backdrop_paints_every_cell() {
        let palette = Palette::solarized_dark();
        let mut terminal = Terminal::new(TestBackend::new(40, 6)).unwrap();
        terminal
            .draw(|frame| backdrop(frame, frame.area(), &palette))
            .unwrap();
        let buf = terminal.backend().buffer();
        for y in 0..6 {
            for x in 0..40 {
                assert_eq!(buf.cell((x, y)).unwrap().bg, color(palette.bg));
            }
        }
    }
}
