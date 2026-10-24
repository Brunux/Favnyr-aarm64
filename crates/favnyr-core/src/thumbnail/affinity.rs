//! Bounded extraction of embedded PNG previews from Affinity documents.

use std::path::Path;

use super::{Thumbnail, from_encoded};

const PNG_SIGNATURE: &[u8; 8] = b"\x89PNG\r\n\x1a\n";
const AFFINITY_SCAN_BUFFER_BYTES: usize = 64 * 1024;
const MAX_AFFINITY_SCAN_BYTES: u64 = 512 * 1024 * 1024;
const MAX_AFFINITY_PNG_SPAN_BYTES: u64 = 50 * 1024 * 1024;
const MAX_AFFINITY_THUMBNAIL_BYTES: u64 = 16 * 1024 * 1024;
const MAX_AFFINITY_PNG_CHUNKS: usize = 4096;
const MAX_AFFINITY_PNG_PROBES: usize = 4096;
const MAX_AFFINITY_PNG_CANDIDATES: usize = 32;

#[derive(Clone, Copy)]
struct EmbeddedPng {
    offset: u64,
    len: u64,
}

pub(super) fn is_affinity_extension(extension: &str) -> bool {
    ["af", "afdesign", "afphoto", "afpub"]
        .iter()
        .any(|candidate| extension.eq_ignore_ascii_case(candidate))
}

/// Extracts the smallest structurally valid PNG embedded in an Affinity file.
/// The format is proprietary, so this path deliberately relies only on PNG's
/// public chunk structure and never attempts to interpret document records.
/// Scanning is streamed and bounded; candidates are allocated one at a time.
pub(super) fn from_affinity_thumbnail(path: &Path, max_px: u32) -> Option<Thumbnail> {
    use std::io::{Read, Seek, SeekFrom};

    let mut file = std::fs::File::open(path).ok()?;
    let scan_end = file.metadata().ok()?.len().min(MAX_AFFINITY_SCAN_BYTES);
    let candidates = affinity_png_candidates(&mut file, scan_end).ok()?;
    for candidate in candidates {
        let encoded_len = usize::try_from(candidate.len).ok()?;
        file.seek(SeekFrom::Start(candidate.offset)).ok()?;
        let mut encoded = vec![0u8; encoded_len];
        file.read_exact(&mut encoded).ok()?;
        if let Some(thumbnail) = from_encoded(&encoded, max_px) {
            return Some(thumbnail);
        }
    }
    None
}

fn affinity_png_candidates(
    file: &mut std::fs::File,
    scan_end: u64,
) -> std::io::Result<Vec<EmbeddedPng>> {
    let mut candidates = Vec::new();
    let mut cursor = 0u64;
    let mut probes = 0usize;
    while probes < MAX_AFFINITY_PNG_PROBES
        && scan_end.saturating_sub(cursor) >= PNG_SIGNATURE.len() as u64
    {
        let Some(offset) = find_next_png_signature(file, cursor, scan_end)? else {
            break;
        };
        probes += 1;
        if let Some(end) = embedded_png_end(file, offset, scan_end)? {
            let len = end - offset;
            if len <= MAX_AFFINITY_THUMBNAIL_BYTES {
                retain_small_affinity_candidate(&mut candidates, EmbeddedPng { offset, len });
            }
            cursor = end;
        } else {
            cursor = offset + 1;
        }
    }
    candidates.sort_unstable_by_key(|candidate| (candidate.len, candidate.offset));
    Ok(candidates)
}

fn find_next_png_signature(
    file: &mut std::fs::File,
    start: u64,
    scan_end: u64,
) -> std::io::Result<Option<u64>> {
    use std::io::{Read, Seek, SeekFrom};

    file.seek(SeekFrom::Start(start))?;
    let mut buffer = [0u8; AFFINITY_SCAN_BUFFER_BYTES];
    let mut cursor = start;
    let mut matched = 0usize;
    while cursor < scan_end {
        let remaining = usize::try_from((scan_end - cursor).min(buffer.len() as u64))
            .expect("the scan buffer length always fits usize");
        let read = file.read(&mut buffer[..remaining])?;
        if read == 0 {
            break;
        }
        for (index, byte) in buffer[..read].iter().enumerate() {
            if *byte == PNG_SIGNATURE[matched] {
                matched += 1;
                if matched == PNG_SIGNATURE.len() {
                    let end = cursor + index as u64 + 1;
                    return Ok(Some(end - PNG_SIGNATURE.len() as u64));
                }
            } else {
                matched = usize::from(*byte == PNG_SIGNATURE[0]);
            }
        }
        cursor += read as u64;
    }
    Ok(None)
}

fn embedded_png_end(
    file: &mut std::fs::File,
    start: u64,
    scan_end: u64,
) -> std::io::Result<Option<u64>> {
    use std::io::{Read, Seek, SeekFrom};

    let Some(mut cursor) = start.checked_add(PNG_SIGNATURE.len() as u64) else {
        return Ok(None);
    };
    file.seek(SeekFrom::Start(cursor))?;
    for chunk_index in 0..MAX_AFFINITY_PNG_CHUNKS {
        let Some(header_end) = cursor.checked_add(8) else {
            return Ok(None);
        };
        if header_end > scan_end {
            return Ok(None);
        }

        let mut header = [0u8; 8];
        file.read_exact(&mut header)?;
        let data_len = u64::from(u32::from_be_bytes(
            header[..4].try_into().expect("four-byte PNG length"),
        ));
        let chunk_type: &[u8; 4] = header[4..].try_into().expect("four-byte PNG chunk type");
        if !chunk_type.iter().all(u8::is_ascii_alphabetic) {
            return Ok(None);
        }
        if chunk_index == 0 && (chunk_type != b"IHDR" || data_len != 13) {
            return Ok(None);
        }

        let Some(chunk_end) = header_end
            .checked_add(data_len)
            .and_then(|end| end.checked_add(4))
        else {
            return Ok(None);
        };
        if chunk_end > scan_end || chunk_end - start > MAX_AFFINITY_PNG_SPAN_BYTES {
            return Ok(None);
        }
        if chunk_type == b"IEND" {
            return Ok((data_len == 0).then_some(chunk_end));
        }

        cursor = chunk_end;
        file.seek(SeekFrom::Start(cursor))?;
    }
    Ok(None)
}

fn retain_small_affinity_candidate(candidates: &mut Vec<EmbeddedPng>, candidate: EmbeddedPng) {
    if candidates.len() < MAX_AFFINITY_PNG_CANDIDATES {
        candidates.push(candidate);
        return;
    }
    let Some((largest_index, largest)) = candidates
        .iter()
        .enumerate()
        .max_by_key(|(_, existing)| (existing.len, existing.offset))
    else {
        return;
    };
    if candidate.len < largest.len {
        candidates[largest_index] = candidate;
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_support::{encoded_cover, tempdir};
    use super::super::{from_image, generate};
    use super::*;
    use crate::fs::FileKind;

    fn patterned_png(width: u32, height: u32) -> Vec<u8> {
        let image = image::RgbaImage::from_fn(width, height, |x, y| {
            image::Rgba([
                x.wrapping_mul(17) as u8,
                y.wrapping_mul(29) as u8,
                x.wrapping_mul(11).wrapping_add(y.wrapping_mul(7)) as u8,
                255,
            ])
        });
        let mut cursor = std::io::Cursor::new(Vec::new());
        image::DynamicImage::ImageRgba8(image)
            .write_to(&mut cursor, image::ImageFormat::Png)
            .unwrap();
        cursor.into_inner()
    }

    #[test]
    fn affinity_thumbnail_streams_across_buffers_and_selects_the_smallest_png() {
        let dir = tempdir();
        let path = dir.join("drawing.AFPHOTO");
        let large = patterned_png(128, 64);
        let small = encoded_cover(image::ImageFormat::Png, [240, 30, 10, 255]);
        assert!(small.len() < large.len());

        // Start the first signature three bytes before the scan-buffer edge to
        // exercise matching across reads, then place the actual thumbnail later.
        let mut affinity = vec![0x55; AFFINITY_SCAN_BUFFER_BYTES - 3];
        affinity.extend_from_slice(&large);
        affinity.extend_from_slice(b"proprietary document records");
        affinity.extend_from_slice(&small);
        std::fs::write(&path, affinity).unwrap();

        let thumb = from_image(&path, 12).expect("embedded Affinity thumbnail");
        assert_eq!((thumb.width, thumb.height), (12, 6));
        assert_eq!(&thumb.rgba[..4], &[240, 30, 10, 255]);
        assert!(generate(&path, FileKind::Image, 12).is_some());

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn malformed_affinity_png_signature_falls_back_without_decoding() {
        let dir = tempdir();
        for extension in ["af", "afdesign", "afphoto", "afpub"] {
            let path = dir.join(format!("broken.{extension}"));
            let mut bytes = b"Affinity-like prefix".to_vec();
            bytes.extend_from_slice(PNG_SIGNATURE);
            bytes.extend_from_slice(&13u32.to_be_bytes());
            bytes.extend_from_slice(b"IDAT"); // a PNG must begin with IHDR
            bytes.extend_from_slice(&[0; 17]);
            std::fs::write(&path, bytes).unwrap();
            assert!(from_image(&path, 32).is_none(), "{extension}");
        }

        std::fs::remove_dir_all(&dir).ok();
    }
}
