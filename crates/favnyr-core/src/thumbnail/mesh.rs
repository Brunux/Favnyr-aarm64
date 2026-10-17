//! Shared bounded triangle rendering and buffered text input for model previews.
use super::Thumbnail;
use std::io::BufRead;
use std::time::{Duration, Instant};

pub(super) const TIME_LIMIT: Duration = Duration::from_secs(10);
pub(super) const MAX_TRIANGLES: u64 = 20_000_000;
pub(super) const RETAINED_TRIANGLES: usize = 65_536;
pub(super) const MAX_SIDE: u32 = 256;
pub(super) const MAX_PIXEL_VISITS: u64 = 64_000_000;
pub(super) type Triangle = [[f32; 3]; 3];
pub(super) type Consumer<'a> = dyn FnMut(Triangle) -> Option<()> + 'a;

/// Each visit must replay the same geometry. Small meshes are retained to
/// avoid a second visit; large meshes use bounded streaming passes.
pub(super) fn render(
    max_px: u32,
    deadline: Instant,
    mut visit: impl FnMut(&mut Consumer<'_>) -> Option<()>,
) -> Option<Thumbnail> {
    if max_px == 0 || Instant::now() >= deadline {
        return None;
    }
    let mut min = [f64::INFINITY; 3];
    let mut max = [f64::NEG_INFINITY; 3];
    let mut retained = Some(Vec::new());
    let mut count = 0u64;
    visit(&mut |triangle| {
        check_triangle(&triangle, &mut count, deadline)?;
        for vertex in triangle {
            for (axis, value) in project(vertex.map(f64::from)).into_iter().enumerate() {
                min[axis] = min[axis].min(value);
                max[axis] = max[axis].max(value);
            }
        }
        if let Some(triangles) = &mut retained {
            if triangles.len() < RETAINED_TRIANGLES {
                triangles.push(triangle);
            } else {
                retained = None;
            }
        }
        Some(())
    })?;
    let extent = (max[0] - min[0]).max(max[1] - min[1]);
    if count == 0 || !extent.is_finite() || extent <= 0.0 {
        return None;
    }
    let side = max_px.min(MAX_SIDE) as usize;
    let mut raster = Raster {
        side: side * 2,
        rgba: vec![0; side * side * 16],
        depth: vec![f32::NEG_INFINITY; side * side * 4],
        pixel_visits: 0,
    };
    let center: [f64; 3] = std::array::from_fn(|axis| (min[axis] + max[axis]) * 0.5);
    let scale = raster.side as f64 * 0.88 / extent;
    let mut drawn = 0u64;
    let mut draw = |triangle: Triangle| {
        check_triangle(&triangle, &mut drawn, deadline)?;
        let points = triangle.map(|vertex| {
            let p = project(vertex.map(f64::from));
            std::array::from_fn(|axis| ((p[axis] - center[axis]) * scale) as f32)
        });
        raster.draw(points)
    };
    if let Some(triangles) = retained {
        for triangle in triangles {
            draw(triangle)?;
        }
    } else {
        visit(&mut draw)?;
    }
    if drawn != count || Instant::now() >= deadline {
        return None;
    }
    raster.finish()
}

fn check_triangle(triangle: &Triangle, count: &mut u64, deadline: Instant) -> Option<()> {
    *count += 1;
    if *count > MAX_TRIANGLES
        || (*count % 256 == 1 && Instant::now() >= deadline)
        || !triangle.iter().flatten().all(|value| value.is_finite())
    {
        return None;
    }
    Some(())
}

/// Convert a right-handed Y-up mesh to the renderer's Z-up coordinates.
pub(super) fn y_up(point: [f32; 3]) -> [f32; 3] {
    [point[0], -point[2], point[1]]
}

/// Most lines borrow the input buffer. Only a line crossing a buffer boundary
/// is copied, and a missing newline can never grow an unbounded allocation.
pub(super) fn lines<R: BufRead>(
    reader: &mut R,
    length: u64,
    max_line_bytes: usize,
    deadline: Instant,
    mut consume: impl FnMut(&str) -> Option<()>,
) -> Option<()> {
    let mut partial = Vec::new();
    let mut read = 0u64;
    loop {
        if Instant::now() >= deadline {
            return None;
        }
        let bytes = reader.fill_buf().ok()?;
        if bytes.is_empty() {
            break;
        }
        read = read.checked_add(bytes.len() as u64)?;
        if read > length {
            return None;
        }
        let mut start = 0;
        if !partial.is_empty() {
            start = bytes
                .iter()
                .position(|byte| *byte == b'\n')
                .map_or(bytes.len(), |index| index + 1);
            if partial.len() + start > max_line_bytes {
                return None;
            }
            partial.extend_from_slice(&bytes[..start]);
            if partial.last() == Some(&b'\n') {
                consume(std::str::from_utf8(&partial).ok()?)?;
                partial.clear();
            }
        }
        let remaining = &bytes[start..];
        // Validate a buffer once, then use the standard library's optimized
        // newline search. A UTF-8 model name can straddle the buffer boundary.
        let valid = match std::str::from_utf8(remaining) {
            Ok(text) => text,
            Err(error) if error.error_len().is_none() => {
                std::str::from_utf8(&remaining[..error.valid_up_to()]).ok()?
            }
            Err(_) => return None,
        };
        for line in valid.split_inclusive('\n') {
            if line.len() > max_line_bytes {
                return None;
            }
            if line.ends_with('\n') {
                consume(line)?;
            } else {
                partial.extend_from_slice(line.as_bytes());
            }
        }
        partial.extend_from_slice(&remaining[valid.len()..]);
        if partial.len() > max_line_bytes {
            return None;
        }
        let used = bytes.len();
        reader.consume(used);
    }
    if read != length {
        return None;
    }
    if !partial.is_empty() {
        consume(std::str::from_utf8(&partial).ok()?)?;
    }
    Some(())
}

// A fixed Z-up camera, slightly above and to the right of the front face.
// Double precision before normalization accommodates very small/large units
// and models translated far from the origin without overflowing f32 math.
fn project(point: [f64; 3]) -> [f64; 3] {
    let [x, y, z] = point;
    let c = 3.0f64.sqrt() * 0.5;
    [
        c * x + 0.5 * y,
        -0.25 * x + 0.5 * c * y + c * z,
        0.5 * c * x - 0.75 * y + 0.5 * z,
    ]
}

struct Raster {
    side: usize,
    rgba: Vec<u8>,
    depth: Vec<f32>,
    pixel_visits: u64,
}

impl Raster {
    fn draw(&mut self, mut p: [[f32; 3]; 3]) -> Option<()> {
        let a: [f32; 3] = std::array::from_fn(|axis| p[1][axis] - p[0][axis]);
        let b: [f32; 3] = std::array::from_fn(|axis| p[2][axis] - p[0][axis]);
        let mut normal = [
            a[1] * b[2] - a[2] * b[1],
            a[2] * b[0] - a[0] * b[2],
            a[0] * b[1] - a[1] * b[0],
        ];
        if !normal.iter().all(|v| v.is_finite()) || normal[2].abs() < 1e-12 {
            return Some(());
        }
        // Mesh normals are often absent or stale. Recompute from geometry,
        // lighting both windings so an inverted mesh still has a useful preview.
        if normal[2] < 0.0 {
            normal = normal.map(|value| -value);
        }
        let norm = normal.iter().map(|value| value * value).sum::<f32>().sqrt();
        let diffuse =
            ((-0.35 * normal[0] + 0.45 * normal[1] + 0.82 * normal[2]) / norm).clamp(0.0, 1.0);
        let shade = 0.3 + 0.7 * diffuse;
        let color = [
            (195.0 * shade) as u8,
            (210.0 * shade) as u8,
            (225.0 * shade) as u8,
            255,
        ];
        for vertex in &mut p {
            vertex[0] += self.side as f32 * 0.5;
            vertex[1] = self.side as f32 * 0.5 - vertex[1];
        }
        let mut area = edge(p[0], p[1], p[2][0], p[2][1]);
        if area < 0.0 {
            p.swap(1, 2);
            area = -area;
        }
        if area <= 0.0 {
            return Some(());
        }
        let low: [usize; 2] = std::array::from_fn(|axis| {
            (p.iter().map(|v| v[axis]).fold(f32::INFINITY, f32::min) - 0.5)
                .ceil()
                .clamp(0.0, self.side as f32) as usize
        });
        let high: [usize; 2] = std::array::from_fn(|axis| {
            (p.iter().map(|v| v[axis]).fold(f32::NEG_INFINITY, f32::max) - 0.5)
                .floor()
                .clamp(-1.0, self.side as f32 - 1.0) as isize
        })
        .map(|v| (v + 1) as usize);
        if high[0] <= low[0] || high[1] <= low[1] {
            return Some(());
        }
        self.pixel_visits += ((high[0] - low[0]) * (high[1] - low[1])) as u64;
        if self.pixel_visits > MAX_PIXEL_VISITS {
            return None;
        }
        let inverse_area = 1.0 / area;
        for y in low[1]..high[1] {
            for x in low[0]..high[0] {
                let e0 = edge(p[1], p[2], x as f32 + 0.5, y as f32 + 0.5);
                let e1 = edge(p[2], p[0], x as f32 + 0.5, y as f32 + 0.5);
                let e2 = edge(p[0], p[1], x as f32 + 0.5, y as f32 + 0.5);
                if e0 < 0.0 || e1 < 0.0 || e2 < 0.0 {
                    continue;
                }
                let depth = (e0 * p[0][2] + e1 * p[1][2] + e2 * p[2][2]) * inverse_area;
                let index = y * self.side + x;
                if depth > self.depth[index] {
                    self.depth[index] = depth;
                    self.rgba[index * 4..index * 4 + 4].copy_from_slice(&color);
                }
            }
        }
        Some(())
    }

    fn finish(self) -> Option<Thumbnail> {
        let side = self.side / 2;
        let mut rgba = vec![0; side * side * 4];
        let mut visible = false;
        for y in 0..side {
            for x in 0..side {
                let mut sum = [0u32; 4];
                for dy in 0..2 {
                    for dx in 0..2 {
                        let source = ((y * 2 + dy) * self.side + x * 2 + dx) * 4;
                        for (channel, value) in sum.iter_mut().enumerate() {
                            *value += u32::from(self.rgba[source + channel]);
                        }
                    }
                }
                if sum[3] == 0 {
                    continue;
                }
                visible = true;
                let target = (y * side + x) * 4;
                // Average covered colors, then coverage separately. Slint
                // expects straight alpha, including the antialiased outline.
                for channel in 0..3 {
                    rgba[target + channel] = (sum[channel] * 255 / sum[3]) as u8;
                }
                rgba[target + 3] = ((sum[3] + 2) / 4) as u8;
            }
        }
        visible.then_some(Thumbnail {
            width: side as u32,
            height: side as u32,
            rgba,
        })
    }
}

fn edge(a: [f32; 3], b: [f32; 3], x: f32, y: f32) -> f32 {
    (b[0] - a[0]) * (y - a[1]) - (b[1] - a[1]) * (x - a[0])
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn overdraw_is_bounded() {
        let mut raster = Raster {
            side: 4,
            rgba: vec![0; 64],
            depth: vec![f32::NEG_INFINITY; 16],
            pixel_visits: MAX_PIXEL_VISITS,
        };
        assert!(
            raster
                .draw([[-2.0, -2.0, 0.0], [2.0, -2.0, 0.0], [0.0, 2.0, 0.0]])
                .is_none()
        );
    }

    #[test]
    fn depth_testing_is_independent_of_submission_order() {
        let back = [[-10.0, -10.0, 0.0], [10.0, -10.0, 0.0], [0.0, 10.0, 0.0]];
        let front = [[-5.0, -5.0, 8.0], [5.0, -5.0, 5.0], [0.0, 5.0, 8.0]];
        let draw = |triangles: [Triangle; 2]| {
            let mut raster = Raster {
                side: 32,
                rgba: vec![0; 32 * 32 * 4],
                depth: vec![f32::NEG_INFINITY; 32 * 32],
                pixel_visits: 0,
            };
            for triangle in triangles {
                raster.draw(triangle).unwrap();
            }
            raster.finish().unwrap()
        };
        assert_eq!(draw([front, back]), draw([back, front]));
    }

    #[test]
    #[ignore = "requires a local directory containing OBJ and glTF samples"]
    fn local_mesh_samples_render() {
        let directory = std::env::var_os("FAVNYR_TEST_MESH_DIR").expect("model directory");
        let mut paths: Vec<_> = std::fs::read_dir(directory)
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .filter(|path| {
                path.extension().is_some_and(|extension| {
                    ["obj", "glb", "gltf"]
                        .iter()
                        .any(|ext| extension.eq_ignore_ascii_case(ext))
                })
            })
            .collect();
        paths.sort();
        assert!(!paths.is_empty());
        for (index, path) in paths.iter().enumerate() {
            let started = Instant::now();
            let thumbnail = super::super::generate(path, crate::fs::FileKind::File, 256);
            eprintln!(
                "Model sample {}: {:.3}s, rendered={}",
                path.file_name().unwrap().to_string_lossy(),
                started.elapsed().as_secs_f64(),
                thumbnail.is_some()
            );
            let thumbnail = thumbnail.expect("model preview");
            if let Some(output) = std::env::var_os("FAVNYR_TEST_OUTPUT_DIR") {
                image::save_buffer(
                    Path::new(&output).join(format!("preview_model_{index}.png")),
                    &thumbnail.rgba,
                    thumbnail.width,
                    thumbnail.height,
                    image::ColorType::Rgba8,
                )
                .unwrap();
            }
        }
    }
}
