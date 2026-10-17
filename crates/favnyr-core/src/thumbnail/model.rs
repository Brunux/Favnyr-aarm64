//! Embedded preview images from 3D document formats.

use std::fs::File;
use std::io::Read;
use std::io::Seek;
use std::path::Path;
use std::process::Command;
use zip::ZipArchive;

use super::Thumbnail;
use super::decode::{downscale, from_encoded_bounded};
use super::tools::{SEVEN_ZIP_PROGRAMS, TOOL_TIMEOUT, ToolOutput, no_console, run_bounded_output};

const MAX_RELATIONSHIP_BYTES: usize = 256 * 1024;
const MAX_MODEL_IMAGE_BYTES: usize = 8 * 1024 * 1024;
const MAX_MODEL_IMAGE_PIXELS: u64 = 4_000_000;
const MAX_BLEND_PREFIX_BYTES: usize = 8 * 1024 * 1024;
const MAX_BLEND_PREVIEW_SIDE: u32 = 1024;
const MAX_BLEND_RENDER_BLOCKS: usize = 16;

/// Blender backup files use the same format, with decimal digits appended
/// directly to the extension. No integer parsing or allocation is needed.
pub(super) fn is_blend_extension(extension: &str) -> bool {
    extension
        .get(..5)
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case("blend"))
        && extension.as_bytes()[5..].iter().all(u8::is_ascii_digit)
}

pub(super) fn from_model_preview(path: &Path, max_px: u32) -> Option<Thumbnail> {
    let extension = path.extension()?.to_str()?;
    if extension.eq_ignore_ascii_case("3mf") {
        from_3mf_preview(path, max_px)
    } else if is_blend_extension(extension) {
        from_blend_preview(path, max_px)
    } else {
        None
    }
}

/// Reads bounded decompressed bytes without creating extracted files or a shell.
///
/// Only the compressed Blender stream still needs this: a gzip or zstd file
/// has no container to walk, and decoding either in process would mean a new
/// dependency per algorithm. ZIP packages are read natively.
fn run_7zip(path: &Path, format: &str, limit: usize) -> Option<ToolOutput> {
    for program in SEVEN_ZIP_PROGRAMS {
        let mut command = Command::new(program);
        command.args(["x", "-so", "-y", format]).arg(path);
        no_console(&mut command);
        if let Some(output) = run_bounded_output(command, TOOL_TIMEOUT, limit) {
            return Some(output);
        }
    }
    None
}

/// Reads one named member of a ZIP archive, refusing anything larger than
/// `limit` before it is decompressed.
///
/// The declared size is checked first so a crafted archive cannot make this
/// allocate hundreds of megabytes for a thumbnail, and the read is capped
/// again afterwards in case that declaration lied.
fn read_zip_member(archive: &mut ZipArchive<File>, member: &str, limit: usize) -> Option<Vec<u8>> {
    let entry = archive.by_name(member).ok()?;
    if entry.size() > limit as u64 {
        return None;
    }
    let mut bytes = Vec::with_capacity(entry.size().min(limit as u64) as usize);
    entry.take(limit as u64).read_to_end(&mut bytes).ok()?;
    Some(bytes)
}

/// A 3MF file is a ZIP package, so its thumbnail is read in process.
///
/// This is the one place where an external archiver used to be required for a
/// thumbnail, and it no longer is: the container is parsed here and its one
/// compressed member inflated by `flate2`, already linked for PNG decoding.
/// Nothing is written to disk and no process is started.
fn from_3mf_preview(path: &Path, max_px: u32) -> Option<Thumbnail> {
    let mut file = File::open(path).ok()?;
    let mut signature = [0u8; 4];
    file.read_exact(&mut signature).ok()?;
    if signature != *b"PK\x03\x04" {
        return None;
    }
    file.rewind().ok()?;
    let mut archive = ZipArchive::new(file).ok()?;

    let relationship = read_zip_member(&mut archive, "_rels/.rels", MAX_RELATIONSHIP_BYTES)
        .and_then(|bytes| thumbnail_relationship_target(&bytes));
    if let Some(target) = relationship.as_deref()
        && let Some(thumbnail) = read_zip_member(&mut archive, target, MAX_MODEL_IMAGE_BYTES)
            .and_then(|bytes| from_encoded_bounded(&bytes, max_px, MAX_MODEL_IMAGE_PIXELS))
    {
        return Some(thumbnail);
    }

    // Some producers omit the package relationship but use the conventional
    // thumbnail part. Only the known image parts are tried as a fallback.
    for target in [
        "Metadata/thumbnail.png",
        "Metadata/thumbnail.jpg",
        "Metadata/thumbnail.jpeg",
    ] {
        if relationship.as_deref() == Some(target) {
            continue;
        }
        if let Some(thumbnail) = read_zip_member(&mut archive, target, MAX_MODEL_IMAGE_BYTES)
            .and_then(|bytes| from_encoded_bounded(&bytes, max_px, MAX_MODEL_IMAGE_PIXELS))
        {
            return Some(thumbnail);
        }
    }
    None
}

/// Identifier of the thumbnail relationship inside an OPC package.
///
/// **It is a name, not an address, and nothing ever fetches it.** Open
/// Packaging Conventions, the container 3MF is built on, names relationship
/// types with URIs because a URI is a globally unique identifier. This exact
/// string is what a producer writes in the `Type` attribute of `_rels/.rels`,
/// so recognising the thumbnail means comparing against it character for
/// character. Matching a suffix instead would accept a relationship from some
/// other namespace that happens to end the same way.
const THUMBNAIL_RELATIONSHIP: &str =
    "http://schemas.openxmlformats.org/package/2006/relationships/metadata/thumbnail";

fn thumbnail_relationship_target(xml: &[u8]) -> Option<String> {
    let xml = std::str::from_utf8(xml).ok()?;
    for part in xml.split('<').skip(1) {
        let Some(end) = part.find('>') else {
            continue;
        };
        let tag = &part[..end];
        let name_end = tag
            .find(|ch: char| ch.is_ascii_whitespace() || ch == '/')
            .unwrap_or(tag.len());
        if tag[..name_end].rsplit(':').next() != Some("Relationship") {
            continue;
        }
        if xml_attribute(tag, "Type") != Some(THUMBNAIL_RELATIONSHIP)
            || xml_attribute(tag, "TargetMode") == Some("External")
        {
            continue;
        }
        if let Some(target) = xml_attribute(tag, "Target")
            .and_then(decode_xml_attribute)
            .and_then(|target| normalize_package_target(&target))
        {
            return Some(target);
        }
    }
    None
}

/// Finds an XML attribute while accepting either quote style and any ordering.
fn xml_attribute<'a>(tag: &'a str, key: &str) -> Option<&'a str> {
    let bytes = tag.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        while index < bytes.len() && !bytes[index].is_ascii_alphabetic() {
            index += 1;
        }
        let start = index;
        while index < bytes.len()
            && (bytes[index].is_ascii_alphanumeric() || b"_:.-".contains(&bytes[index]))
        {
            index += 1;
        }
        if start == index {
            break;
        }
        let name = &tag[start..index];
        while index < bytes.len() && bytes[index].is_ascii_whitespace() {
            index += 1;
        }
        if bytes.get(index) != Some(&b'=') {
            continue;
        }
        index += 1;
        while index < bytes.len() && bytes[index].is_ascii_whitespace() {
            index += 1;
        }
        let quote = *bytes.get(index)?;
        if quote != b'\'' && quote != b'"' {
            return None;
        }
        index += 1;
        let value_start = index;
        index += bytes[index..].iter().position(|byte| *byte == quote)?;
        let value = &tag[value_start..index];
        index += 1;
        if name == key {
            return Some(value);
        }
    }
    None
}

fn decode_xml_attribute(value: &str) -> Option<String> {
    let mut result = String::with_capacity(value.len());
    let mut rest = value;
    while let Some(start) = rest.find('&') {
        result.push_str(&rest[..start]);
        rest = &rest[start + 1..];
        let end = rest.find(';')?;
        let character = match &rest[..end] {
            "amp" => '&',
            "quot" => '"',
            "apos" => '\'',
            "lt" => '<',
            "gt" => '>',
            entity if entity.starts_with("#x") => {
                char::from_u32(u32::from_str_radix(&entity[2..], 16).ok()?)?
            }
            entity if entity.starts_with('#') => char::from_u32(entity[1..].parse().ok()?)?,
            _ => return None,
        };
        result.push(character);
        rest = &rest[end + 1..];
    }
    result.push_str(rest);
    Some(result)
}

/// Decode percent-escaped UTF-8 paths once; callers enforce their own policy
/// for archive members or local companion files after decoding.
pub(super) fn decode_uri_path(target: &str) -> Option<String> {
    let mut bytes = Vec::with_capacity(target.len());
    let source = target.as_bytes();
    let mut index = 0;
    while index < source.len() {
        if source[index] == b'%' {
            let hex = source.get(index + 1..index + 3)?;
            let digits = std::str::from_utf8(hex).ok()?;
            bytes.push(u8::from_str_radix(digits, 16).ok()?);
            index += 3;
        } else {
            bytes.push(source[index]);
            index += 1;
        }
    }
    String::from_utf8(bytes).ok()
}

/// Converts an OPC target to an archive member, rejecting paths that could
/// match several members or refer outside the package.
fn normalize_package_target(target: &str) -> Option<String> {
    let decoded = decode_uri_path(target)?;
    let relative = decoded.strip_prefix('/').unwrap_or(&decoded);
    if relative.is_empty()
        || relative.starts_with('/')
        || relative.starts_with('-')
        || relative
            .chars()
            .any(|ch| ch.is_control() || "\\:*?<>|#".contains(ch))
        || relative
            .split('/')
            .any(|part| part.is_empty() || part == "." || part == "..")
    {
        return None;
    }
    let extension = relative.rsplit('.').next()?;
    if !["png", "jpg", "jpeg"]
        .iter()
        .any(|candidate| extension.eq_ignore_ascii_case(candidate))
    {
        return None;
    }
    Some(relative.to_owned())
}

fn from_blend_preview(path: &Path, max_px: u32) -> Option<Thumbnail> {
    let mut file = File::open(path).ok()?;
    let mut magic = [0u8; 4];
    file.read_exact(&mut magic).ok()?;

    let bytes = if &magic == b"BLEN" {
        let mut bytes = Vec::new();
        File::open(path)
            .ok()?
            .take(MAX_BLEND_PREFIX_BYTES as u64)
            .read_to_end(&mut bytes)
            .ok()?;
        bytes
    } else {
        let format = if magic == [0x28, 0xb5, 0x2f, 0xfd] {
            "-tzstd"
        } else if magic[..2] == [0x1f, 0x8b] {
            "-tgzip"
        } else {
            return None;
        };
        run_7zip(path, format, MAX_BLEND_PREFIX_BYTES)?.bytes
    };
    parse_blend_preview(&bytes, max_px)
}

fn parse_blend_preview(bytes: &[u8], max_px: u32) -> Option<Thumbnail> {
    if !bytes.starts_with(b"BLENDER") {
        return None;
    }
    let (mut cursor, block_header, wide_length) =
        if bytes.get(7..12) == Some(b"17-01") && bytes.get(12) == Some(&b'v') {
            (17usize, 32usize, true)
        } else {
            let pointer_size = match bytes.get(7)? {
                b'-' => 8,
                b'_' => 4,
                _ => return None,
            };
            if bytes.get(8) != Some(&b'v') {
                return None;
            }
            (12, 16 + pointer_size, false)
        };
    for _ in 0..MAX_BLEND_RENDER_BLOCKS {
        let header_end = cursor.checked_add(block_header)?;
        let header = bytes.get(cursor..header_end)?;
        let length = if wide_length {
            let raw: [u8; 8] = header.get(16..24)?.try_into().ok()?;
            let length = i64::from_le_bytes(raw);
            usize::try_from(length).ok()?
        } else {
            let raw: [u8; 4] = header.get(4..8)?.try_into().ok()?;
            usize::try_from(i32::from_le_bytes(raw)).ok()?
        };
        let data_end = header_end.checked_add(length)?;
        let data = bytes.get(header_end..data_end)?;
        if &header[..4] == b"REND" {
            cursor = data_end;
            continue;
        }
        if &header[..4] != b"TEST" || length < 8 {
            return None;
        }
        let width_bytes: [u8; 4] = data[..4].try_into().ok()?;
        let height_bytes: [u8; 4] = data[4..8].try_into().ok()?;
        let (width, height) = (
            u32::from_le_bytes(width_bytes),
            u32::from_le_bytes(height_bytes),
        );
        if width == 0
            || height == 0
            || width > MAX_BLEND_PREVIEW_SIDE
            || height > MAX_BLEND_PREVIEW_SIDE
            || u64::from(width) * u64::from(height) * 4 != (length - 8) as u64
        {
            return None;
        }
        let mut image = image::RgbaImage::from_raw(width, height, data[8..].to_vec())?;
        image::imageops::flip_vertical_in_place(&mut image);
        return Some(downscale(image::DynamicImage::ImageRgba8(image), max_px));
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::thumbnail::test_support::{encoded_cover, tempdir};
    use zip::CompressionMethod;

    #[test]
    fn package_relationship_selects_only_internal_thumbnail_images() {
        let xml = format!(
            "<Relationships><Relationship TargetMode='External' Target='external_preview.png' Type='{THUMBNAIL_RELATIONSHIP}'/><Relationship Target='/Metadata/my%20preview&amp;more.png' Type='{THUMBNAIL_RELATIONSHIP}'/></Relationships>"
        );
        assert_eq!(
            thumbnail_relationship_target(xml.as_bytes()).as_deref(),
            Some("Metadata/my preview&more.png")
        );
        assert!(normalize_package_target("/%2e%2e/image.png").is_none());
        assert!(normalize_package_target("/Metadata/*.png").is_none());
        assert!(normalize_package_target("/Metadata/model.xml").is_none());
    }

    #[test]
    fn blend_preview_flips_rows_and_handles_both_block_headers() {
        let pixels = [
            10, 20, 30, 255, 40, 50, 60, 255, 70, 80, 90, 255, 100, 110, 120, 255,
        ];
        let mut data = Vec::new();
        data.extend_from_slice(&2u32.to_le_bytes());
        data.extend_from_slice(&2u32.to_le_bytes());
        data.extend_from_slice(&pixels);

        let mut old = b"BLENDER-v300".to_vec();
        old.extend_from_slice(b"REND");
        old.extend_from_slice(&0u32.to_le_bytes());
        old.extend_from_slice(&[0u8; 16]);
        old.extend_from_slice(b"TEST");
        old.extend_from_slice(&(data.len() as u32).to_le_bytes());
        old.extend_from_slice(&[0u8; 16]);
        old.extend_from_slice(&data);
        let thumbnail = parse_blend_preview(&old, 2).expect("legacy preview");
        assert_eq!((thumbnail.width, thumbnail.height), (2, 2));
        assert_eq!(&thumbnail.rgba[..8], &pixels[8..16]);
        for extension in ["blend", "blend1", "blend2", "BLEND12"] {
            let file = std::env::temp_dir()
                .join(format!("favnyr-model-{}.{extension}", std::process::id()));
            std::fs::write(&file, &old).expect("temporary model written");
            assert!(from_model_preview(&file, 2).is_some());
            std::fs::remove_file(&file).expect("temporary model removed");
        }
        old[8] = b'V';
        assert!(parse_blend_preview(&old, 2).is_none());

        let mut recent = b"BLENDER17-01v0500".to_vec();
        recent.extend_from_slice(b"TEST");
        recent.extend_from_slice(&[0u8; 12]);
        recent.extend_from_slice(&(data.len() as u64).to_le_bytes());
        recent.extend_from_slice(&[0u8; 8]);
        recent.extend_from_slice(&data);
        assert!(parse_blend_preview(&recent, 2).is_some());
        recent[33..41].copy_from_slice(&u64::MAX.to_le_bytes());
        assert!(parse_blend_preview(&recent, 2).is_none());
    }

    /// Builds a 3MF package in memory: a ZIP holding the relationship file
    /// and the image it points at.
    fn write_3mf(path: &Path, target: &str, rels: Option<&str>, method: CompressionMethod) {
        use std::io::Write;
        let file = File::create(path).expect("create package");
        let mut writer = zip::ZipWriter::new(file);
        let options: zip::write::FileOptions<'_, '_, ()> =
            zip::write::FileOptions::default().compression_method(method);

        if let Some(rels) = rels {
            writer.start_file("_rels/.rels", options).expect("rels");
            writer.write_all(rels.as_bytes()).expect("rels body");
        }
        writer.start_file(target, options).expect("image");
        writer
            .write_all(&encoded_cover(image::ImageFormat::Png, [10, 20, 30, 255]))
            .expect("image body");
        writer.finish().expect("package");
    }

    #[test]
    fn a_3mf_thumbnail_is_read_without_an_external_archiver() {
        let dir = tempdir();
        let rels = format!(
            r#"<?xml version="1.0"?><Relationships><Relationship Type="{THUMBNAIL_RELATIONSHIP}" Target="/Metadata/custom_preview.png"/></Relationships>"#
        );

        // Both storage methods a producer may choose. Deflate is the one that
        // used to require an external tool.
        for (name, method) in [
            ("stored.3mf", CompressionMethod::Stored),
            ("deflated.3mf", CompressionMethod::Deflated),
        ] {
            let package = dir.join(name);
            write_3mf(&package, "Metadata/custom_preview.png", Some(&rels), method);
            assert!(
                from_3mf_preview(&package, 8).is_some(),
                "{name} yielded no thumbnail"
            );
        }
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn a_3mf_without_a_relationship_falls_back_to_the_conventional_part() {
        let dir = tempdir();
        let package = dir.join("conventional.3mf");
        // No `_rels/.rels` at all, which some producers omit.
        write_3mf(
            &package,
            "Metadata/thumbnail.png",
            None,
            CompressionMethod::Deflated,
        );
        assert!(from_3mf_preview(&package, 8).is_some());
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn a_package_without_any_known_thumbnail_part_yields_nothing() {
        let dir = tempdir();
        let package = dir.join("empty.3mf");
        write_3mf(
            &package,
            "Metadata/unrelated.png",
            None,
            CompressionMethod::Deflated,
        );
        assert!(from_3mf_preview(&package, 8).is_none());
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn something_that_is_not_an_archive_is_refused_before_anything_is_parsed() {
        let dir = tempdir();
        let package = dir.join("not_an_archive.3mf");
        std::fs::write(&package, b"plain text, no signature").expect("write");
        assert!(from_3mf_preview(&package, 8).is_none());
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    #[ignore = "requires local model samples, and a recent 7-Zip for the blend half"]
    fn local_samples_have_embedded_previews() {
        let archive = std::env::var_os("FAVNYR_TEST_3MF").expect("3MF sample path");
        let blend = std::env::var_os("FAVNYR_TEST_BLEND").expect("blend sample path");
        let archive = from_3mf_preview(Path::new(&archive), 256).expect("3MF thumbnail");
        let blend = from_blend_preview(Path::new(&blend), 256).expect("blend thumbnail");
        assert!(archive.width > 0 && archive.height > 0);
        assert!(blend.width > 0 && blend.height > 0);
        if let Some(output) = std::env::var_os("FAVNYR_TEST_OUTPUT_DIR") {
            let output = Path::new(&output);
            for (name, thumbnail) in [("preview_3mf.png", archive), ("preview_blend.png", blend)] {
                image::save_buffer(
                    output.join(name),
                    &thumbnail.rgba,
                    thumbnail.width,
                    thumbnail.height,
                    image::ColorType::Rgba8,
                )
                .expect("preview image written");
            }
        }
    }
}
