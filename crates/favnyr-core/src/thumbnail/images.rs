//! Raster image previews and inexpensive header metadata.

use std::path::Path;

use super::Thumbnail;
use super::affinity::{from_affinity_thumbnail, is_affinity_extension};
use super::decode::{decode_bounded, downscale};
use super::psd::from_psd_thumbnail;

/// Metadata of an image read from the HEADER only (no pixel decoding):
/// `(width, height, color bits per pixel, alpha present)`. When an
/// alpha channel exists, its depth is subtracted from the third element: an
/// RGBA8 image therefore returns `(width, height, 24, true)`. `None` if the
/// format isn't handled by `image` or the file is unreadable. Fast
/// (a few KB read) → suited to filling columns on the fly.
pub fn image_meta(path: &Path) -> Option<(u32, u32, u16, bool)> {
    use image::ImageDecoder;
    let decoder = image::ImageReader::open(path)
        .ok()?
        .with_guessed_format()
        .ok()?
        .into_decoder()
        .ok()?;
    let (w, h) = decoder.dimensions();
    let color = decoder.color_type();
    let has_alpha = color.has_alpha();
    let total_bits = color.bits_per_pixel();
    // The ColorType variants exposed by `image` have channels of equal depth
    // (RGBA8, RGBA16, RGBA32F, LA8, LA16). Removing one channel therefore gives
    // the correct color depth, without counting alpha twice in the display.
    let color_bits = if has_alpha {
        total_bits - total_bits / u16::from(color.channel_count())
    } else {
        total_bits
    };
    Some((w, h, color_bits, has_alpha))
}

/// Thumbnail of an image file (pure-Rust decoding). `None` if the format
/// isn't supported / the file is unreadable.
pub fn from_image(path: &Path, max_px: u32) -> Option<Thumbnail> {
    let extension = path
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or_default();
    if extension.eq_ignore_ascii_case("psd") {
        return from_psd_thumbnail(path, max_px);
    }
    if is_affinity_extension(extension) {
        return from_affinity_thumbnail(path, max_px);
    }

    let reader = image::ImageReader::open(path)
        .ok()?
        .with_guessed_format()
        .ok()?;
    Some(downscale(decode_bounded(reader)?, max_px))
}

#[cfg(test)]
mod tests {
    use super::super::test_support::tempdir;
    use super::*;

    #[test]
    fn image_thumbnail_preserves_aspect_and_bounds() {
        let dir = tempdir();
        let path = dir.join("red.png");
        // 100×50 opaque red, encoded as PNG via `image`.
        let buf = image::RgbaImage::from_pixel(100, 50, image::Rgba([255, 0, 0, 255]));
        buf.save(&path).unwrap();

        let thumb = from_image(&path, 32).expect("decode");
        // 2:1 aspect preserved, bounded to 32 → 32×16.
        assert!(thumb.width <= 32 && thumb.height <= 32);
        assert_eq!(thumb.width, 32);
        assert_eq!(thumb.height, 16);
        assert_eq!(thumb.rgba.len() as u32, thumb.width * thumb.height * 4);
        // Red pixel preserved.
        assert_eq!(&thumb.rgba[0..4], &[255, 0, 0, 255]);

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn image_meta_reads_dimensions_and_depth() {
        let dir = tempdir();
        let path = dir.join("rgba.png");
        // 64×32 RGBA8: 24 color bits + one separate alpha channel.
        image::RgbaImage::from_pixel(64, 32, image::Rgba([1, 2, 3, 4]))
            .save(&path)
            .unwrap();
        let (w, h, bits, alpha) = image_meta(&path).expect("metadata readable");
        assert_eq!((w, h), (64, 32));
        assert_eq!(bits, 24);
        assert!(alpha);
        // A non-image file → None.
        let txt = dir.join("x.txt");
        std::fs::write(&txt, b"nope").unwrap();
        assert!(image_meta(&txt).is_none());
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn unsupported_path_returns_none() {
        let dir = tempdir();
        let path = dir.join("not-an-image.txt");
        std::fs::write(&path, b"hello").unwrap();
        assert!(from_image(&path, 32).is_none());
        std::fs::remove_dir_all(&dir).ok();
    }
}
