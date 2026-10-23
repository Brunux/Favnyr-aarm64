//! Readability: WCAG relative luminance and the contrast ratio it gives.
//!
//! A theme that cannot be read is not a theme, so a palette is validated before
//! it is written anywhere (`Theme::validate`). The floor is the one WCAG puts on
//! body text (4.5:1) for the text role and 3:1 - the "large text and UI
//! components" floor - for everything a surface uses as a signal.

use crate::Rgb;

/// WCAG relative luminance, 0.0 (black) to 1.0 (white).
pub fn luminance(colour: Rgb) -> f32 {
    let channel = |value: u8| {
        let value = f32::from(value) / 255.0;
        if value <= 0.039_28 {
            value / 12.92
        } else {
            ((value + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * channel(colour.r) + 0.7152 * channel(colour.g) + 0.0722 * channel(colour.b)
}

/// The contrast ratio between two colours, 1.0 (identical) to 21.0 (black on
/// white). Symmetric, as the definition is.
pub fn ratio(a: Rgb, b: Rgb) -> f32 {
    let (first, second) = (luminance(a), luminance(b));
    let (lighter, darker) = if first > second {
        (first, second)
    } else {
        (second, first)
    };
    (lighter + 0.05) / (darker + 0.05)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_ends_of_the_scale_are_the_definition() {
        assert!(ratio(Rgb::new(0, 0, 0), Rgb::new(255, 255, 255)) > 20.9);
        assert!((ratio(Rgb::new(255, 255, 255), Rgb::new(255, 255, 255)) - 1.0).abs() < 1e-6);
        assert!((ratio(Rgb::new(0, 0, 0), Rgb::new(0, 0, 0)) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn the_ratio_is_symmetric_and_the_luminance_is_not() {
        let (a, b) = (Rgb::new(0x2a, 0xa1, 0x98), Rgb::new(0x00, 0x2b, 0x36));
        assert!((ratio(a, b) - ratio(b, a)).abs() < 1e-6);
        assert!(luminance(a) > luminance(b));
    }

    #[test]
    fn the_sessions_pairs_are_where_the_floor_says_they_are() {
        let bg = Rgb::new(0x00, 0x2b, 0x36);
        let text = Rgb::new(0x93, 0xa1, 0xa1);
        let accent = Rgb::new(0x2a, 0xa1, 0x98);
        assert!(ratio(text, bg) >= 4.5, "solarized text clears body text");
        assert!(ratio(accent, bg) >= 3.0, "solarized cyan clears UI");
    }
}
