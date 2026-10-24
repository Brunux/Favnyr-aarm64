//! Embedded MP3 and FLAC cover art, without audio stream decoding.

use std::path::Path;

use super::{Thumbnail, from_encoded};

/// Maximum amount of audio metadata inspected for one thumbnail. Album covers
/// are normally far smaller; the cap prevents a malformed file from making
/// several workers allocate or seek through an amount derived from arbitrary
/// bytes.
const MAX_AUDIO_METADATA_BYTES: usize = 16 * 1024 * 1024;
const MAX_AUDIO_PICTURES: usize = 16;
const MAX_FLAC_METADATA_BLOCKS: usize = 128;

/// Thumbnail from embedded audio cover art. Only formats with a small, bounded
/// parser live here: ID3v2 for MP3 and native `PICTURE` blocks for FLAC.
pub fn from_audio(path: &Path, max_px: u32) -> Option<Thumbnail> {
    let extension = path
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or_default();
    if extension.eq_ignore_ascii_case("mp3") {
        from_id3_cover(path, max_px)
    } else if extension.eq_ignore_ascii_case("flac") {
        from_flac_cover(path, max_px)
    } else {
        None
    }
}

fn from_id3_cover(path: &Path, max_px: u32) -> Option<Thumbnail> {
    let (version, flags, mut tag) = read_id3v2_tag(path)?;
    // ID3v2.2 compression applies to the entire tag and requires a decompressor.
    // It is rare and intentionally rejected rather than misparsed.
    if version == 2 && flags & 0x40 != 0 {
        return None;
    }
    let tag_unsynchronized = flags & 0x80 != 0;
    if tag_unsynchronized {
        tag = remove_id3_unsynchronization(&tag);
    }

    let mut cursor = id3_frames_start(version, flags, &tag)?;
    let mut fallback = None;
    let mut pictures_checked = 0usize;
    while cursor < tag.len() {
        let Some((id, size, frame_flags, header_len)) = id3_frame_header(version, &tag[cursor..])
        else {
            break;
        };
        if id.iter().all(|byte| *byte == 0) {
            break; // padding after the final frame
        }
        if !id
            .iter()
            .all(|byte| byte.is_ascii_uppercase() || byte.is_ascii_digit())
        {
            break; // corrupt boundary: never search arbitrary audio bytes
        }
        let Some(data_start) = cursor.checked_add(header_len) else {
            break;
        };
        let Some(data_end) = data_start.checked_add(size) else {
            break;
        };
        let Some(raw_frame) = tag.get(data_start..data_end) else {
            break;
        };
        cursor = data_end;

        let is_picture = (version == 2 && id == b"PIC") || (version != 2 && id == b"APIC");
        if !is_picture {
            continue;
        }
        pictures_checked += 1;
        if pictures_checked > MAX_AUDIO_PICTURES {
            break;
        }
        let Some(frame) = prepare_id3_frame(version, frame_flags, raw_frame, tag_unsynchronized)
        else {
            continue;
        };
        let Some((picture_type, encoded)) = id3_picture_data(version, &frame) else {
            continue;
        };
        // ID3 picture type 3 is the front cover. Tags may contain artist,
        // leaflet or back-cover images before it, so only those are fallbacks.
        if picture_type == 3 {
            if let Some(thumbnail) = from_encoded(encoded, max_px) {
                return Some(thumbnail);
            }
        } else if fallback.is_none() {
            fallback = from_encoded(encoded, max_px);
        }
    }
    fallback
}

/// Reads native FLAC metadata until the last block, without touching audio
/// frames. Non-picture blocks are skipped directly in the file; only bounded
/// `PICTURE` payloads are allocated and decoded.
fn from_flac_cover(path: &Path, max_px: u32) -> Option<Thumbnail> {
    use std::io::{Read, Seek, SeekFrom};

    let mut file = std::fs::File::open(path).ok()?;
    let mut magic = [0u8; 4];
    file.read_exact(&mut magic).ok()?;
    if &magic != b"fLaC" {
        return None;
    }

    let mut inspected_bytes = magic.len();
    let mut blocks_checked = 0usize;
    let mut pictures_checked = 0usize;
    let mut fallback = None;
    loop {
        if blocks_checked >= MAX_FLAC_METADATA_BLOCKS {
            return fallback;
        }
        blocks_checked += 1;

        let mut header = [0u8; 4];
        if file.read_exact(&mut header).is_err() {
            return fallback;
        }
        let is_last = header[0] & 0x80 != 0;
        let block_type = header[0] & 0x7f;
        let block_len =
            (usize::from(header[1]) << 16) | (usize::from(header[2]) << 8) | usize::from(header[3]);
        let Some(next_inspected) = inspected_bytes
            .checked_add(header.len())
            .and_then(|value| value.checked_add(block_len))
        else {
            return fallback;
        };
        if next_inspected > MAX_AUDIO_METADATA_BYTES {
            return fallback;
        }
        inspected_bytes = next_inspected;

        if block_type == 6 {
            pictures_checked += 1;
            if pictures_checked > MAX_AUDIO_PICTURES {
                return fallback;
            }
            let mut block = vec![0u8; block_len];
            if file.read_exact(&mut block).is_err() {
                return fallback;
            }
            if let Some((picture_type, encoded)) = flac_picture_data(&block) {
                // FLAC uses the same picture-type registry as ID3: type 3 is
                // the front cover, while other valid pictures are fallbacks.
                if picture_type == 3 {
                    if let Some(thumbnail) = from_encoded(encoded, max_px) {
                        return Some(thumbnail);
                    }
                } else if fallback.is_none() {
                    fallback = from_encoded(encoded, max_px);
                }
            }
        } else if file.seek(SeekFrom::Current(block_len as i64)).is_err() {
            return fallback;
        }

        if is_last {
            return fallback;
        }
    }
}

fn flac_picture_data(block: &[u8]) -> Option<(u32, &[u8])> {
    fn read_u32(block: &[u8], cursor: &mut usize) -> Option<u32> {
        let end = cursor.checked_add(4)?;
        let value = u32::from_be_bytes(block.get(*cursor..end)?.try_into().ok()?);
        *cursor = end;
        Some(value)
    }

    fn read_counted<'a>(block: &'a [u8], cursor: &mut usize) -> Option<&'a [u8]> {
        let len = usize::try_from(read_u32(block, cursor)?).ok()?;
        let end = cursor.checked_add(len)?;
        let value = block.get(*cursor..end)?;
        *cursor = end;
        Some(value)
    }

    let mut cursor = 0usize;
    let picture_type = read_u32(block, &mut cursor)?;
    let mime = read_counted(block, &mut cursor)?;
    if mime == b"-->" {
        return None; // external URL: Favnyr never fetches remote artwork
    }
    let _description = read_counted(block, &mut cursor)?;
    for _ in 0..4 {
        let _ = read_u32(block, &mut cursor)?; // width, height, depth, palette size
    }
    let encoded = read_counted(block, &mut cursor)?;
    if encoded.is_empty() || cursor != block.len() {
        return None;
    }
    Some((picture_type, encoded))
}

fn read_id3v2_tag(path: &Path) -> Option<(u8, u8, Vec<u8>)> {
    use std::io::Read;

    let mut file = std::fs::File::open(path).ok()?;
    let mut header = [0u8; 10];
    file.read_exact(&mut header).ok()?;
    if &header[..3] != b"ID3" || !matches!(header[3], 2..=4) {
        return None;
    }
    let size = synchsafe_u32(&header[6..10])? as usize;
    if size == 0 || size > MAX_AUDIO_METADATA_BYTES {
        return None;
    }
    let mut tag = vec![0u8; size];
    file.read_exact(&mut tag).ok()?;
    Some((header[3], header[5], tag))
}

/// Decodes a 28-bit synchsafe integer. The top bit of every source byte must
/// be clear; accepting it would let malformed sizes jump across frame bounds.
fn synchsafe_u32(bytes: &[u8]) -> Option<u32> {
    let bytes: &[u8; 4] = bytes.try_into().ok()?;
    if bytes.iter().any(|byte| byte & 0x80 != 0) {
        return None;
    }
    Some(
        bytes
            .iter()
            .fold(0, |value, byte| (value << 7) | u32::from(*byte)),
    )
}

fn remove_id3_unsynchronization(bytes: &[u8]) -> Vec<u8> {
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        decoded.push(bytes[index]);
        if bytes[index] == 0xff && bytes.get(index + 1) == Some(&0) {
            index += 1;
        }
        index += 1;
    }
    decoded
}

fn id3_frames_start(version: u8, flags: u8, tag: &[u8]) -> Option<usize> {
    if flags & 0x40 == 0 {
        return Some(0);
    }
    match version {
        3 => {
            let size = u32::from_be_bytes(tag.get(..4)?.try_into().ok()?) as usize;
            4usize.checked_add(size).filter(|end| *end <= tag.len())
        }
        4 => {
            // In v2.4 the extended-header size includes these four bytes.
            let size = synchsafe_u32(tag.get(..4)?)? as usize;
            (size >= 4 && size <= tag.len()).then_some(size)
        }
        _ => None,
    }
}

/// `(frame id, payload size, format flags, header length)`.
fn id3_frame_header(version: u8, bytes: &[u8]) -> Option<(&[u8], usize, u8, usize)> {
    match version {
        2 => {
            let header = bytes.get(..6)?;
            let size = (usize::from(header[3]) << 16)
                | (usize::from(header[4]) << 8)
                | usize::from(header[5]);
            Some((&header[..3], size, 0, 6))
        }
        3 | 4 => {
            let header = bytes.get(..10)?;
            let size = if version == 4 {
                synchsafe_u32(&header[4..8])? as usize
            } else {
                u32::from_be_bytes(header[4..8].try_into().ok()?) as usize
            };
            Some((&header[..4], size, header[9], 10))
        }
        _ => None,
    }
}

/// Removes optional per-frame prefixes and unsynchronization. Compressed or
/// encrypted frames are rejected: silently interpreting them as images would
/// be incorrect and these uncommon tags can safely keep the audio icon.
fn prepare_id3_frame<'a>(
    version: u8,
    flags: u8,
    raw: &'a [u8],
    tag_unsynchronized: bool,
) -> Option<std::borrow::Cow<'a, [u8]>> {
    let mut prefix = 0usize;
    let frame_unsynchronized = match version {
        2 => false,
        3 => {
            if flags & (0x80 | 0x40) != 0 {
                return None; // compression or encryption
            }
            if flags & 0x20 != 0 {
                prefix = 1; // grouping identity
            }
            false
        }
        4 => {
            if flags & (0x08 | 0x04) != 0 {
                return None; // compression or encryption
            }
            if flags & 0x40 != 0 {
                prefix += 1; // grouping identity
            }
            if flags & 0x01 != 0 {
                prefix += 4; // data length indicator
            }
            flags & 0x02 != 0
        }
        _ => return None,
    };
    let payload = raw.get(prefix..)?;
    if frame_unsynchronized && !tag_unsynchronized {
        Some(std::borrow::Cow::Owned(remove_id3_unsynchronization(
            payload,
        )))
    } else {
        Some(std::borrow::Cow::Borrowed(payload))
    }
}

fn id3_picture_data(version: u8, frame: &[u8]) -> Option<(u8, &[u8])> {
    let encoding = *frame.first()?;
    if encoding > 3 {
        return None;
    }
    let mut cursor = 1usize;
    if version == 2 {
        cursor = cursor.checked_add(3)?; // fixed `PNG`/`JPG` image format
    } else {
        // MIME is always ISO-8859-1 and null-terminated, independently from
        // the description's text encoding.
        cursor =
            cursor.checked_add(frame.get(cursor..)?.iter().position(|byte| *byte == 0)? + 1)?;
    }
    let picture_type = *frame.get(cursor)?;
    cursor += 1;
    let description = frame.get(cursor..)?;
    let terminator_len = if matches!(encoding, 1 | 2) { 2 } else { 1 };
    let description_len = if terminator_len == 1 {
        description.iter().position(|byte| *byte == 0)?
    } else {
        description
            .chunks_exact(2)
            .position(|pair| pair == [0, 0])?
            * 2
    };
    let image_start = cursor.checked_add(description_len + terminator_len)?;
    let image = frame.get(image_start..)?;
    (!image.is_empty()).then_some((picture_type, image))
}

#[cfg(test)]
mod tests {
    use super::super::generate;
    use super::super::test_support::{encoded_cover, tempdir};
    use super::*;
    use crate::fs::FileKind;

    fn synchsafe_bytes(value: usize) -> [u8; 4] {
        assert!(value < (1 << 28));
        [
            ((value >> 21) & 0x7f) as u8,
            ((value >> 14) & 0x7f) as u8,
            ((value >> 7) & 0x7f) as u8,
            (value & 0x7f) as u8,
        ]
    }

    fn id3v23_frame(id: &[u8; 4], payload: &[u8]) -> Vec<u8> {
        let mut frame = id.to_vec();
        frame.extend_from_slice(&(payload.len() as u32).to_be_bytes());
        frame.extend_from_slice(&[0, 0]); // status + format flags
        frame.extend_from_slice(payload);
        frame
    }

    fn id3v23_picture(picture_type: u8, encoded: &[u8]) -> Vec<u8> {
        let mut payload = vec![0]; // ISO-8859-1 description
        payload.extend_from_slice(b"image/png\0");
        payload.push(picture_type);
        payload.push(0); // empty description
        payload.extend_from_slice(encoded);
        id3v23_frame(b"APIC", &payload)
    }

    fn write_id3_file(path: &Path, version: u8, flags: u8, tag: &[u8]) {
        let mut file = b"ID3".to_vec();
        file.extend_from_slice(&[version, 0, flags]);
        file.extend_from_slice(&synchsafe_bytes(tag.len()));
        file.extend_from_slice(tag);
        file.extend_from_slice(&[0xff, 0xfb, 0x90, 0x64]); // inert MPEG-like tail
        std::fs::write(path, file).unwrap();
    }

    fn flac_picture(picture_type: u32, encoded: &[u8]) -> Vec<u8> {
        let mut block = picture_type.to_be_bytes().to_vec();
        block.extend_from_slice(&(b"image/png".len() as u32).to_be_bytes());
        block.extend_from_slice(b"image/png");
        block.extend_from_slice(&0u32.to_be_bytes()); // empty description
        block.extend_from_slice(&20u32.to_be_bytes()); // width
        block.extend_from_slice(&10u32.to_be_bytes()); // height
        block.extend_from_slice(&32u32.to_be_bytes()); // color depth
        block.extend_from_slice(&0u32.to_be_bytes()); // not indexed
        block.extend_from_slice(&(encoded.len() as u32).to_be_bytes());
        block.extend_from_slice(encoded);
        block
    }

    fn write_flac_file(path: &Path, blocks: &[(u8, Vec<u8>)]) {
        assert!(!blocks.is_empty());
        let mut file = b"fLaC".to_vec();
        for (index, (block_type, payload)) in blocks.iter().enumerate() {
            assert!(*block_type < 0x80 && payload.len() <= 0x00ff_ffff);
            let last = index + 1 == blocks.len();
            file.push(*block_type | if last { 0x80 } else { 0 });
            file.extend_from_slice(&[
                ((payload.len() >> 16) & 0xff) as u8,
                ((payload.len() >> 8) & 0xff) as u8,
                (payload.len() & 0xff) as u8,
            ]);
            file.extend_from_slice(payload);
        }
        file.extend_from_slice(&[0xff, 0xf8, 0x69]); // inert FLAC-like audio tail
        std::fs::write(path, file).unwrap();
    }

    #[test]
    fn mp3_thumbnail_prefers_the_front_cover_without_decoding_audio() {
        let dir = tempdir();
        let path = dir.join("song.mp3");
        let blue = encoded_cover(image::ImageFormat::Png, [0, 0, 255, 255]);
        let red = encoded_cover(image::ImageFormat::Png, [255, 0, 0, 255]);
        let mut tag = id3v23_picture(4, &blue); // back cover comes first
        tag.extend_from_slice(&id3v23_picture(3, &red)); // front cover wins
        write_id3_file(&path, 3, 0, &tag);

        let thumb = from_audio(&path, 12).expect("embedded front cover");
        assert_eq!((thumb.width, thumb.height), (12, 6));
        assert_eq!(&thumb.rgba[..4], &[255, 0, 0, 255]);
        assert!(generate(&path, FileKind::Audio, 12).is_some());

        // Dispatch remains strict even if another extension happens to begin
        // with bytes that look like an ID3 tag.
        let flac = dir.join("song.flac");
        std::fs::copy(&path, &flac).unwrap();
        assert!(from_audio(&flac, 12).is_none());
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn flac_thumbnail_prefers_front_cover_and_skips_audio_frames() {
        let dir = tempdir();
        let path = dir.join("song.flac");
        let blue = encoded_cover(image::ImageFormat::Png, [0, 0, 255, 255]);
        let red = encoded_cover(image::ImageFormat::Png, [255, 0, 0, 255]);
        write_flac_file(
            &path,
            &[
                (0, vec![0; 34]),            // mandatory STREAMINFO
                (6, flac_picture(4, &blue)), // back cover comes first
                (6, flac_picture(3, &red)),  // front cover wins
            ],
        );

        let thumb = from_audio(&path, 12).expect("embedded FLAC front cover");
        assert_eq!((thumb.width, thumb.height), (12, 6));
        assert_eq!(&thumb.rgba[..4], &[255, 0, 0, 255]);
        assert!(generate(&path, FileKind::Audio, 12).is_some());

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn malformed_or_coverless_flac_falls_back_without_large_allocation() {
        let dir = tempdir();
        let coverless = dir.join("coverless.flac");
        write_flac_file(&coverless, &[(0, vec![0; 34])]);
        assert!(from_audio(&coverless, 32).is_none());

        // The largest representable FLAC block would cross Favnyr's bounded
        // metadata budget. Its payload is deliberately absent: rejection must
        // happen from the four-byte header, before any allocation or read.
        let oversized = dir.join("oversized.flac");
        std::fs::write(&oversized, b"fLaC\x86\xff\xff\xff").unwrap();
        assert!(from_audio(&oversized, 32).is_none());

        // A linked PICTURE block must be rejected by its MIME type, without
        // interpreting or following its external reference.
        let mut remote = 3u32.to_be_bytes().to_vec();
        remote.extend_from_slice(&3u32.to_be_bytes());
        remote.extend_from_slice(b"-->");
        remote.extend_from_slice(&0u32.to_be_bytes());
        remote.extend_from_slice(&[0; 16]);
        let reference = b"external_cover";
        remote.extend_from_slice(&(reference.len() as u32).to_be_bytes());
        remote.extend_from_slice(reference);
        assert!(flac_picture_data(&remote).is_none());

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn mp3_thumbnail_supports_id3v22_pic_and_tag_unsynchronization() {
        let dir = tempdir();

        let png = encoded_cover(image::ImageFormat::Png, [12, 34, 56, 255]);
        let mut pic_payload = vec![0];
        pic_payload.extend_from_slice(b"PNG");
        pic_payload.push(3);
        pic_payload.push(0);
        pic_payload.extend_from_slice(&png);
        let mut pic_frame = b"PIC".to_vec();
        let size = pic_payload.len();
        pic_frame.extend_from_slice(&[
            ((size >> 16) & 0xff) as u8,
            ((size >> 8) & 0xff) as u8,
            (size & 0xff) as u8,
        ]);
        pic_frame.extend_from_slice(&pic_payload);
        let v22 = dir.join("old.mp3");
        write_id3_file(&v22, 2, 0, &pic_frame);
        assert!(from_audio(&v22, 16).is_some());

        // JPEG contains 0xff marker bytes. ID3 tag-level unsynchronization
        // inserts a zero after each one; decoding must remove it before image
        // detection while keeping the original APIC frame size meaningful.
        let jpeg = encoded_cover(image::ImageFormat::Jpeg, [90, 140, 190, 255]);
        let frame = id3v23_picture(3, &jpeg);
        let mut unsynchronized = Vec::with_capacity(frame.len());
        for byte in frame {
            unsynchronized.push(byte);
            if byte == 0xff {
                unsynchronized.push(0);
            }
        }
        let v23_unsync = dir.join("unsync.mp3");
        write_id3_file(&v23_unsync, 3, 0x80, &unsynchronized);
        assert!(from_audio(&v23_unsync, 16).is_some());

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn malformed_or_coverless_mp3_falls_back_without_large_allocation() {
        let dir = tempdir();
        let coverless = dir.join("coverless.mp3");
        let title = id3v23_frame(b"TIT2", b"\0A song");
        write_id3_file(&coverless, 3, 0, &title);
        assert!(from_audio(&coverless, 32).is_none());

        let oversized = dir.join("oversized.mp3");
        let mut header = b"ID3".to_vec();
        header.extend_from_slice(&[3, 0, 0]);
        header.extend_from_slice(&synchsafe_bytes(MAX_AUDIO_METADATA_BYTES + 1));
        std::fs::write(&oversized, header).unwrap();
        assert!(from_audio(&oversized, 32).is_none());

        std::fs::remove_dir_all(&dir).ok();
    }
}
