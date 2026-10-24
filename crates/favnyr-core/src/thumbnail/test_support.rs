//! Shared synthetic image fixtures for thumbnail unit tests.

pub(super) fn tempdir() -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!(
        "favnyr-thumb-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    ));
    std::fs::create_dir_all(&d).unwrap();
    d
}

pub(super) fn encoded_cover(format: image::ImageFormat, rgba: [u8; 4]) -> Vec<u8> {
    let image = image::RgbaImage::from_pixel(20, 10, image::Rgba(rgba));
    let mut cursor = std::io::Cursor::new(Vec::new());
    image::DynamicImage::ImageRgba8(image)
        .write_to(&mut cursor, format)
        .unwrap();
    cursor.into_inner()
}
