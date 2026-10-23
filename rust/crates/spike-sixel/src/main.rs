//! Sixel spike: the one technical risk of `docs/plan-ui.md` §6, validated first.
//!
//! It draws two pictures in a real foot window through `ratatui-image`: the
//! 48 px mark from the LazyOS theme (what a row or a hero card shows) and a
//! document picture (what the preview pane and the file browser show), and it
//! prints the protocol the terminal answered so a headless run proves the
//! detection too.
//!
//! ```text
//! spike-sixel [ICON] [PICTURE]
//! ```
//!
//! The frame stays on screen for 30 s, closes on any key, and leaves
//! `/tmp/opencode/spike-drawn` behind so a harness can `grim` the window
//! before it goes away.
//!
//! The pictures use `StatefulImage` (fit-to-area at render time), not the
//! fixed `Image` widget: `Image` silently refuses an area smaller than the
//! protocol it was built for, which is how the first run drew the icon and an
//! empty rectangle beside it.

use std::error::Error;
use std::io;
use std::time::{Duration, Instant};

use ratatui::backend::CrosstermBackend;
use ratatui::crossterm::event::{self, Event, KeyEventKind};
use ratatui::crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::crossterm::{cursor, execute};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::{Frame, Terminal};
use ratatui_image::picker::Picker;
use ratatui_image::protocol::StatefulProtocol;
use ratatui_image::{FontSize, Resize, StatefulImage};

// The solarized values the session already uses (base03, base1, base01, cyan).
const BG: Color = Color::Rgb(0x00, 0x2b, 0x36);
const FG: Color = Color::Rgb(0x93, 0xa1, 0xa1);
const DIM: Color = Color::Rgb(0x58, 0x6e, 0x75);
const ACCENT: Color = Color::Rgb(0x2a, 0xa1, 0x98);

const MARKER: &str = "/tmp/opencode/spike-drawn";

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = std::env::args().skip(1);
    let icon_path = args
        .next()
        .unwrap_or_else(|| "uis/sway/files/usr/share/icons/LazyOS/48x48/apps/lazyos.png".into());
    let photo_path = args
        .next()
        .unwrap_or_else(|| "docs/screenshots/hayato-r1-sway.png".into());

    // Leave the terminal usable even if drawing panics.
    let hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen, cursor::Show);
        hook(info);
    }));

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, cursor::Hide)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout))?;
    terminal.clear()?;

    // The terminal answers with the protocols it speaks and its cell size.
    let picker = Picker::from_query_stdio()?;
    let protocol = format!("{:?}", picker.protocol_type());
    let font = picker.font_size();

    let icon = image::ImageReader::open(&icon_path)?.decode()?;
    let photo = image::ImageReader::open(&photo_path)?.decode()?;
    let mut icon_proto = picker.new_resize_protocol(icon);
    let mut photo_proto = picker.new_resize_protocol(photo);

    let mut draw_ms = draw_once(
        &mut terminal,
        &mut icon_proto,
        &mut photo_proto,
        &protocol,
        &icon_path,
        &photo_path,
    )?;
    write_marker(&protocol, font, &terminal, draw_ms)?;

    // Thirty seconds is enough for a harness to capture; a key ends it sooner.
    // A resize re-fits both pictures, which is what a window manager does to a
    // floating window after it appears.
    let deadline = Instant::now() + Duration::from_secs(30);
    let mut next_repaint = Instant::now() + Duration::from_secs(1);
    while let Some(left) = deadline.checked_duration_since(Instant::now()) {
        if event::poll(left.min(Duration::from_millis(250)))? {
            let ev = event::read()?;
            log_event(&format!("{ev:?}"));
            match ev {
                Event::Key(key) if key.kind == KeyEventKind::Press => break,
                Event::Resize(_, _) => {
                    draw_ms = draw_once(
                        &mut terminal,
                        &mut icon_proto,
                        &mut photo_proto,
                        &protocol,
                        &icon_path,
                        &photo_path,
                    )?;
                    write_marker(&protocol, font, &terminal, draw_ms)?;
                }
                _ => {}
            }
        }
        // A repaint at an unchanged size proves what a ticking UI costs with a
        // picture on screen: the protocol is cached, so only the diff is sent.
        if Instant::now() >= next_repaint {
            draw_ms = draw_once(
                &mut terminal,
                &mut icon_proto,
                &mut photo_proto,
                &protocol,
                &icon_path,
                &photo_path,
            )?;
            write_marker(&protocol, font, &terminal, draw_ms)?;
            log_event(&format!(
                "repaint={draw_ms}ms at {}x{}",
                terminal.size()?.width,
                terminal.size()?.height
            ));
            next_repaint += Duration::from_secs(3);
        }
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen, cursor::Show)?;
    terminal.show_cursor()?;
    println!("protocol: {protocol}");
    Ok(())
}

type Term = Terminal<CrosstermBackend<io::Stdout>>;

/// One frame, timed: `StatefulImage` resizes and encodes the pictures inside
/// the draw, so the number is the whole cost of a picture-heavy screen.
fn draw_once(
    term: &mut Term,
    icon: &mut StatefulProtocol,
    photo: &mut StatefulProtocol,
    protocol: &str,
    icon_path: &str,
    photo_path: &str,
) -> io::Result<u128> {
    let started = Instant::now();
    term.draw(|f| draw(f, icon, photo, protocol, icon_path, photo_path))?;
    Ok(started.elapsed().as_millis())
}

/// Leaves the facts of this run for the harness (and for `docs/optimization.md`).
fn write_marker(protocol: &str, font: FontSize, term: &Term, draw_ms: u128) -> io::Result<()> {
    let size = term.size()?;
    std::fs::write(
        MARKER,
        format!(
            "protocol={protocol}\nfont={}x{}\nterminal={}x{}\ndraw_ms={draw_ms}\n",
            font.width, font.height, size.width, size.height
        ),
    )
}

/// Every event the terminal delivered, so the harness can see whether a
/// compositor resize reaches the app at all.
fn log_event(line: &str) {
    use std::io::Write as _;
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open("/tmp/opencode/spike-events")
    {
        let _ = writeln!(f, "{line}");
    }
}

fn draw(
    f: &mut Frame,
    icon: &mut StatefulProtocol,
    photo: &mut StatefulProtocol,
    protocol: &str,
    icon_path: &str,
    photo_path: &str,
) {
    let area = f.area();
    f.render_widget(Block::default().style(Style::default().bg(BG)), area);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(2),
        ])
        .split(area);

    let header = Paragraph::new(Line::from(vec![
        Span::styled(
            " spike-sixel ",
            Style::default()
                .fg(BG)
                .bg(ACCENT)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            "  foot + ratatui-image · the terminal answered ",
            Style::default().fg(DIM),
        ),
        Span::styled(
            protocol,
            Style::default().fg(FG).add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!("  ·  {}x{} cells", area.width, area.height),
            Style::default().fg(DIM),
        ),
    ]))
    .block(
        Block::default()
            .borders(Borders::BOTTOM)
            .border_style(Style::default().fg(DIM)),
    );
    f.render_widget(header, rows[0]);

    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
        .split(rows[1]);

    let icon_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(DIM))
        .title(Span::styled(" theme icon ", Style::default().fg(ACCENT)));
    f.render_widget(&icon_block, cols[0]);
    let icon_inner = icon_block.inner(cols[0]);
    // The picture gets the box, the two caption rows sit under it; Resize::Fit
    // letterboxes the square mark inside whatever is left.
    let caption_rows = 3;
    let icon_area = Rect::new(
        icon_inner.x,
        icon_inner.y,
        icon_inner.width,
        icon_inner.height.saturating_sub(caption_rows),
    );
    f.render_stateful_widget(
        StatefulImage::new().resize(Resize::Fit(None)),
        icon_area,
        icon,
    );
    f.render_widget(
        Paragraph::new(vec![
            Line::from(Span::styled(
                "  48x48, the launcher",
                Style::default().fg(FG),
            )),
            Line::from(Span::styled(
                "  and the AI hero card",
                Style::default().fg(DIM),
            )),
        ]),
        Rect::new(
            icon_inner.x,
            icon_inner.y + icon_inner.height.saturating_sub(2),
            icon_inner.width,
            2,
        ),
    );

    let photo_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(DIM))
        .title(Span::styled(
            " a document picture, fitted to the pane ",
            Style::default().fg(ACCENT),
        ));
    f.render_widget(&photo_block, cols[1]);
    let photo_inner = photo_block.inner(cols[1]);
    f.render_stateful_widget(
        StatefulImage::new().resize(Resize::Fit(None)),
        photo_inner,
        photo,
    );

    let footer = Paragraph::new(Line::from(vec![
        Span::styled(
            format!("  {}  ·  {}", shorten(icon_path), shorten(photo_path)),
            Style::default().fg(DIM),
        ),
        Span::styled("  ·  any key closes ", Style::default().fg(FG)),
    ]));
    f.render_widget(footer, rows[2]);
}

fn shorten(path: &str) -> String {
    let parts: Vec<&str> = path.split('/').collect();
    if parts.len() > 3 {
        format!("…/{}", parts[parts.len() - 2..].join("/"))
    } else {
        path.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shorten_keeps_a_short_path() {
        assert_eq!(shorten("lazyos.png"), "lazyos.png");
        assert_eq!(shorten("apps/lazyos.png"), "apps/lazyos.png");
        assert_eq!(shorten("usr/apps/lazyos.png"), "usr/apps/lazyos.png");
    }

    #[test]
    fn shorten_elides_an_absolute_path() {
        assert_eq!(
            shorten("/usr/share/icons/LazyOS/48x48/apps/lazyos.png"),
            "…/apps/lazyos.png"
        );
    }

    /// Relative luminance, WCAG 2.x.
    fn luma(color: Color) -> f32 {
        let Color::Rgb(r, g, b) = color else {
            return 0.0;
        };
        let channel = |value: u8| {
            let value = f32::from(value) / 255.0;
            if value <= 0.040_45 {
                value / 12.92
            } else {
                ((value + 0.055) / 1.055).powf(2.4)
            }
        };
        (0.2126 * channel(r)) + (0.7152 * channel(g)) + (0.0722 * channel(b))
    }

    #[test]
    fn text_on_background_is_readable() {
        let (light, dark) = (luma(FG), luma(BG));
        let ratio = (light.max(dark) + 0.05) / (light.min(dark) + 0.05);
        assert!(ratio >= 4.5, "solarized base1 on base03 is {ratio:.2}:1");
    }

    #[test]
    fn the_three_roles_are_distinct() {
        assert_ne!(BG, FG);
        assert_ne!(BG, ACCENT);
        assert_ne!(FG, ACCENT);
    }
}
