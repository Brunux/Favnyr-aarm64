//! Bounded STL parsing feeding the shared CPU renderer.

use std::fs::File;
use std::io::{BufRead, BufReader, Read, Seek, SeekFrom};
use std::path::Path;
use std::time::Instant;

use super::Thumbnail;
use super::mesh::{self, MAX_TRIANGLES, TIME_LIMIT, Triangle, lines};
#[cfg(test)]
use super::mesh::{MAX_SIDE, RETAINED_TRIANGLES};

const MAX_FILE_BYTES: u64 = 4 * 1024 * 1024 * 1024;
const MAX_LINE_BYTES: usize = 4096;

#[derive(Clone, Copy)]
enum Format {
    Binary(u32),
    Ascii,
}

pub(super) fn from_stl(path: &Path, max_px: u32) -> Option<Thumbnail> {
    let deadline = Instant::now() + TIME_LIMIT;
    let file = File::open(path).ok()?;
    let metadata = file.metadata().ok()?;
    if !metadata.is_file() || metadata.len() > MAX_FILE_BYTES || max_px == 0 {
        return None;
    }
    let mut reader = BufReader::with_capacity(64 * 1024, file);
    let format = detect_format(&mut reader, metadata.len())?;
    let result = render(&mut reader, format, metadata.len(), max_px, deadline)?;
    // A writer replacing the contents between the bounds and drawing passes
    // must not leave a thumbnail based on two different versions of the mesh.
    let after = reader.get_ref().metadata().ok()?;
    if after.len() != metadata.len() || after.modified().ok() != metadata.modified().ok() {
        return None;
    }
    Some(result)
}

fn detect_format<R: Read + Seek>(reader: &mut R, length: u64) -> Option<Format> {
    let mut header = [0u8; 84];
    let size = usize::try_from(length.min(84)).ok()?;
    reader.read_exact(&mut header[..size]).ok()?;
    reader.rewind().ok()?;
    if size == 84 {
        let count = u32::from_le_bytes(header[80..84].try_into().ok()?);
        // A binary header may also begin with "solid". Its record count and
        // exact length take precedence over the textual signature.
        if count > 0 && 84 + u64::from(count) * 50 == length {
            return (u64::from(count) <= MAX_TRIANGLES).then_some(Format::Binary(count));
        }
    }
    let first = header[..size]
        .split(|byte| byte.is_ascii_whitespace())
        .find(|s| !s.is_empty())?;
    (first == b"solid").then_some(Format::Ascii)
}

/// Visits every triangle, keeping allocations independent of the file size.
fn visit<R: BufRead + Seek>(
    reader: &mut R,
    format: Format,
    length: u64,
    deadline: Instant,
    mut consume: impl FnMut(Triangle) -> Option<()>,
) -> Option<u64> {
    let mut count = 0u64;
    let mut emit = |triangle: Triangle| {
        count += 1;
        if count > MAX_TRIANGLES || (count % 1024 == 1 && Instant::now() >= deadline) {
            return None;
        }
        consume(triangle)
    };
    match format {
        Format::Binary(triangles) => {
            reader.seek(SeekFrom::Start(84)).ok()?;
            let mut record = [0u8; 50];
            for _ in 0..triangles {
                reader.read_exact(&mut record).ok()?;
                let mut triangle = [[0.0; 3]; 3];
                for (coordinate, bytes) in triangle
                    .iter_mut()
                    .flatten()
                    .zip(record[12..48].chunks_exact(4))
                {
                    *coordinate = f32::from_le_bytes(bytes.try_into().ok()?);
                }
                emit(triangle)?;
            }
        }
        Format::Ascii => {
            reader.rewind().ok()?;
            let mut state = 0;
            let mut triangle = [[0.0; 3]; 3];
            lines(reader, length, MAX_LINE_BYTES, deadline, |line| {
                let mut words = line.split_ascii_whitespace();
                let Some(word) = words.next() else {
                    return Some(());
                };
                match (state, word) {
                    (0 | 8, "solid") => state = 1,
                    (1, "facet") if words.next() == Some("normal") => state = 2,
                    (1, "endsolid") => state = 8,
                    (2, "outer") if words.next() == Some("loop") => state = 3,
                    (3..=5, "vertex") => {
                        for coordinate in &mut triangle[state - 3] {
                            *coordinate = words.next()?.parse().ok()?;
                        }
                        if words.next().is_some() {
                            return None;
                        }
                        state += 1;
                    }
                    (6, "endloop") => state = 7,
                    (7, "endfacet") => {
                        emit(triangle)?;
                        state = 1;
                    }
                    _ => return None,
                }
                Some(())
            })?;
            if state != 8 {
                return None;
            }
        }
    }
    (count > 0 && Instant::now() < deadline).then_some(count)
}

fn render<R: BufRead + Seek>(
    reader: &mut R,
    format: Format,
    length: u64,
    max_px: u32,
    deadline: Instant,
) -> Option<Thumbnail> {
    mesh::render(max_px, deadline, |consume| {
        visit(reader, format, length, deadline, consume).map(|_| ())
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    const FACE: Triangle = [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]];

    fn binary(triangles: &[Triangle]) -> Vec<u8> {
        let mut bytes = vec![0; 80];
        bytes[..5].copy_from_slice(b"solid");
        bytes.extend_from_slice(&(triangles.len() as u32).to_le_bytes());
        for triangle in triangles {
            bytes.extend_from_slice(&[0; 12]);
            for coordinate in triangle.iter().flatten() {
                bytes.extend_from_slice(&coordinate.to_le_bytes());
            }
            bytes.extend_from_slice(&[0; 2]);
        }
        bytes
    }

    fn ascii(triangle: Triangle) -> Vec<u8> {
        let mut text = String::from("solid model\r\nfacet normal 0 0 0\r\nouter loop\r\n");
        for [x, y, z] in triangle {
            text.push_str(&format!("  vertex {x:e}\t{y:e} {z:e}\r\n"));
        }
        text.push_str("endloop\r\nendfacet\r\nendsolid model");
        text.into_bytes()
    }

    fn preview(bytes: &[u8], side: u32) -> Option<Thumbnail> {
        let mut reader = BufReader::with_capacity(37, Cursor::new(bytes));
        let format = detect_format(&mut reader, bytes.len() as u64)?;
        render(
            &mut reader,
            format,
            bytes.len() as u64,
            side,
            Instant::now() + TIME_LIMIT,
        )
    }

    #[test]
    fn binary_and_ascii_match_with_a_misleading_binary_header() {
        let binary = preview(&binary(&[FACE]), 64).expect("binary preview");
        assert_eq!(Some(binary.clone()), preview(&ascii(FACE), 64));
        assert!(binary.rgba.chunks_exact(4).any(|pixel| pixel[3] == 255));
        assert!(
            binary
                .rgba
                .chunks_exact(4)
                .any(|pixel| pixel[3] > 0 && pixel[3] < 255)
        );
        for pixel in binary.rgba.chunks_exact(4).filter(|pixel| pixel[3] > 0) {
            assert!(pixel[0] > 50, "edge colors use straight alpha");
        }
        assert_eq!(&binary.rgba[..4], &[0, 0, 0, 0]);
    }

    #[test]
    fn invalid_records_and_nonfinite_vertices_are_refused() {
        let mut bytes = binary(&[FACE]);
        bytes.pop();
        assert!(preview(&bytes, 32).is_none());
        let mut bytes = binary(&[FACE]);
        bytes[80..84].copy_from_slice(&u32::MAX.to_le_bytes());
        assert!(preview(&bytes, 32).is_none());
        for value in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            let mut face = FACE;
            face[1][0] = value;
            assert!(preview(&binary(&[face]), 32).is_none());
            assert!(preview(&ascii(face), 32).is_none());
        }
        assert!(preview(&binary(&[[[0.0; 3]; 3]]), 32).is_none());
        assert!(preview(b"solid model\nendsolid model", 32).is_none());
    }

    #[test]
    fn ascii_requires_complete_facets_and_bounds_line_allocations() {
        let original = String::from_utf8(ascii(FACE)).unwrap();
        for token in ["outer loop", "endloop", "endfacet", "endsolid model"] {
            assert!(
                preview(original.replace(token, "").as_bytes(), 32).is_none(),
                "{token}"
            );
        }
        assert!(
            preview(
                format!("solid {}\n", "x".repeat(MAX_LINE_BYTES)).as_bytes(),
                32
            )
            .is_none()
        );
        let mut repeated = ascii(FACE);
        repeated.extend_from_slice(b"\n\n");
        repeated.extend_from_slice(&ascii(FACE));
        assert_eq!(preview(&repeated, 32), preview(&ascii(FACE), 32));
        let named = original.replace("model", "model_\u{754c}_\u{754c}_\u{754c}");
        for capacity in 1..40 {
            let mut reader = BufReader::with_capacity(capacity, Cursor::new(named.as_bytes()));
            let rendered = render(
                &mut reader,
                Format::Ascii,
                named.len() as u64,
                32,
                Instant::now() + TIME_LIMIT,
            );
            assert_eq!(
                rendered,
                preview(&ascii(FACE), 32),
                "buffer size {capacity}"
            );
        }
    }

    #[test]
    fn winding_units_and_translation_do_not_hide_the_model() {
        let normal = preview(&binary(&[FACE]), 64).unwrap();
        let mut reversed = FACE;
        reversed.swap(1, 2);
        assert_eq!(Some(normal.clone()), preview(&binary(&[reversed]), 64));
        for scale in [1e-30f32, 1e30] {
            let transformed = FACE.map(|p| p.map(|v| v * scale + scale * 8.0));
            let rendered = preview(&binary(&[transformed]), 64).unwrap();
            let covered = rendered.rgba.chunks_exact(4).filter(|p| p[3] > 0).count();
            let reference = normal.rgba.chunks_exact(4).filter(|p| p[3] > 0).count();
            assert!(covered.abs_diff(reference) < 8);
        }
    }

    #[test]
    fn large_meshes_replay_without_retaining_all_triangles() {
        let mut triangles = vec![[[0.0; 3]; 3]; RETAINED_TRIANGLES + 1];
        triangles.push(FACE);
        assert_eq!(
            preview(&binary(&triangles), 32),
            preview(&binary(&[FACE]), 32)
        );
    }

    #[test]
    fn time_and_resolution_are_bounded() {
        let bytes = binary(&[FACE]);
        let mut cursor = Cursor::new(&bytes);
        assert!(
            render(
                &mut cursor,
                Format::Binary(1),
                bytes.len() as u64,
                32,
                Instant::now()
            )
            .is_none()
        );
        assert_eq!(preview(&bytes, u32::MAX).unwrap().width, MAX_SIDE);
    }

    #[test]
    #[ignore = "requires a local directory of STL samples"]
    fn local_stl_sample_benchmark() {
        let folder = std::env::var_os("FAVNYR_TEST_STL_DIR").expect("sample directory");
        let mut files: Vec<_> = std::fs::read_dir(folder)
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .filter(|path| {
                path.extension()
                    .is_some_and(|ext| ext.eq_ignore_ascii_case("stl"))
            })
            .collect();
        files.sort();
        assert!(!files.is_empty());
        let mut rendered = 0;
        for (index, path) in files.iter().enumerate() {
            let started = Instant::now();
            let thumbnail = super::super::generate(path, crate::fs::FileKind::File, 256);
            eprintln!(
                "STL sample {}: {} bytes, {:.3}s, rendered={}",
                path.file_name().unwrap().to_string_lossy(),
                path.metadata().unwrap().len(),
                started.elapsed().as_secs_f64(),
                thumbnail.is_some()
            );
            // This manual benchmark also reports files beyond the preview
            // budgets; a refusal is not itself a parser correctness failure.
            let Some(thumbnail) = thumbnail else {
                continue;
            };
            rendered += 1;
            if let Some(output) = std::env::var_os("FAVNYR_TEST_OUTPUT_DIR") {
                image::save_buffer(
                    Path::new(&output).join(format!("preview_stl_{index}.png")),
                    &thumbnail.rgba,
                    thumbnail.width,
                    thumbnail.height,
                    image::ColorType::Rgba8,
                )
                .unwrap();
            }
        }
        assert!(rendered > 0, "at least one sample produced a preview");
    }
}
