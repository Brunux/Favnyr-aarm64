//! Polygonal OBJ previews without material or texture loading.

use std::fs::File;
use std::io::{BufRead, BufReader, Seek};
use std::path::Path;
use std::time::Instant;

use super::{Thumbnail, mesh};

const MAX_FILE_BYTES: u64 = 256 * 1024 * 1024;
const MAX_VERTICES: usize = 2_000_000;
const MAX_FACE_VERTICES: usize = 256;
const MAX_LINE_BYTES: usize = 64 * 1024;
const MAX_EAR_TESTS: usize = 4_000_000;

pub(super) fn from_obj(path: &Path, max_px: u32) -> Option<Thumbnail> {
    let deadline = Instant::now() + mesh::TIME_LIMIT;
    let file = File::open(path).ok()?;
    let before = file.metadata().ok()?;
    if !before.is_file() || before.len() > MAX_FILE_BYTES || max_px == 0 {
        return None;
    }
    let mut reader = BufReader::with_capacity(64 * 1024, file);
    let result = mesh::render(max_px, deadline, |consume| {
        visit(&mut reader, before.len(), deadline, consume)
    })?;
    let after = reader.get_ref().metadata().ok()?;
    (before.len() == after.len() && before.modified().ok() == after.modified().ok())
        .then_some(result)
}

fn visit<R: BufRead + Seek>(
    reader: &mut R,
    length: u64,
    deadline: Instant,
    consume: &mut mesh::Consumer<'_>,
) -> Option<()> {
    reader.rewind().ok()?;
    let mut vertices = Vec::new();
    let mut face = Vec::new();
    let mut continuation = String::new();
    let mut ear_tests = 0;
    let mut line = |text: &str| {
        let mut words = text.split_ascii_whitespace();
        match words.next() {
            Some("v") => {
                if vertices.len() >= MAX_VERTICES {
                    return None;
                }
                let mut point = [0f32; 3];
                for coordinate in &mut point {
                    *coordinate = words.next()?.parse().ok()?;
                }
                // The optional fourth coordinate is homogeneous weight.
                // Six/seven coordinates are commonly used for vertex color.
                let extra: Vec<_> = words.take(5).collect();
                match extra.len() {
                    0 | 3 | 4 => {}
                    1 => {
                        let weight: f32 = extra[0].parse().ok()?;
                        if !weight.is_finite() || weight == 0.0 {
                            return None;
                        }
                        point = point.map(|v| v / weight);
                    }
                    _ => return None,
                }
                if !point.iter().all(|v| v.is_finite()) {
                    return None;
                }
                vertices.push(mesh::y_up(point));
            }
            Some("f") => {
                face.clear();
                for word in words {
                    if face.len() >= MAX_FACE_VERTICES {
                        return None;
                    }
                    let index: i64 = word.split('/').next()?.parse().ok()?;
                    let resolved = if index < 0 {
                        vertices.len() as i64 + index
                    } else {
                        index.checked_sub(1)?
                    };
                    let vertex = *vertices.get(usize::try_from(resolved).ok()?)?;
                    if face.last() != Some(&vertex) {
                        face.push(vertex);
                    }
                }
                if face.len() > 3 && face.first() == face.last() {
                    face.pop();
                }
                triangulate(&face, &mut ear_tests, deadline, consume)?;
            }
            // Curves and parametric surfaces need a different tessellator.
            Some("curv" | "curv2" | "surf") => return None,
            _ => {}
        }
        Some(())
    };
    mesh::lines(reader, length, MAX_LINE_BYTES, deadline, |text| {
        let text = text.split('#').next()?.trim();
        let continued = text.strip_suffix('\\');
        let part = continued.unwrap_or(text);
        if continuation.len() + part.len() + 1 > MAX_LINE_BYTES {
            return None;
        }
        if continued.is_some() || !continuation.is_empty() {
            continuation.push_str(part);
            continuation.push(' ');
            if continued.is_none() {
                line(&continuation)?;
                continuation.clear();
            }
        } else {
            line(part)?;
        }
        Some(())
    })?;
    continuation.is_empty().then_some(())
}

/// Clip ears on the dominant plane instead of filling a concave polygon with
/// a triangle fan. Work is bounded even for adversarial or self-crossing faces.
fn triangulate(
    face: &[[f32; 3]],
    tests: &mut usize,
    deadline: Instant,
    consume: &mut mesh::Consumer<'_>,
) -> Option<()> {
    if face.len() < 3 {
        return None;
    }
    if face.len() == 3 {
        return consume([face[0], face[1], face[2]]);
    }
    let mut normal = [0f64; 3];
    for i in 0..face.len() {
        let a = face[i].map(f64::from);
        let b = face[(i + 1) % face.len()].map(f64::from);
        for axis in 0..3 {
            normal[axis] +=
                (a[(axis + 1) % 3] - b[(axis + 1) % 3]) * (a[(axis + 2) % 3] + b[(axis + 2) % 3]);
        }
    }
    let dropped = (0..3).max_by(|a, b| normal[*a].abs().total_cmp(&normal[*b].abs()))?;
    if normal[dropped] == 0.0 {
        return Some(());
    }
    let points: Vec<[f64; 2]> = face
        .iter()
        .map(|p| {
            [
                f64::from(p[(dropped + 1) % 3]),
                f64::from(p[(dropped + 2) % 3]),
            ]
        })
        .collect();
    let orientation = normal[dropped].signum();
    let mut remaining: Vec<_> = (0..face.len()).collect();
    while remaining.len() > 3 {
        if Instant::now() >= deadline {
            return None;
        }
        let mut clipped = false;
        for i in 0..remaining.len() {
            let a = remaining[(i + remaining.len() - 1) % remaining.len()];
            let b = remaining[i];
            let c = remaining[(i + 1) % remaining.len()];
            let area = cross(points[a], points[b], points[c]) * orientation;
            if area < 0.0 {
                continue;
            }
            let mut occupied = false;
            for &j in &remaining {
                *tests += 1;
                if *tests > MAX_EAR_TESTS {
                    return None;
                }
                if j == a || j == b || j == c {
                    continue;
                }
                if cross(points[a], points[b], points[j]) * orientation >= 0.0
                    && cross(points[b], points[c], points[j]) * orientation >= 0.0
                    && cross(points[c], points[a], points[j]) * orientation >= 0.0
                {
                    occupied = true;
                    break;
                }
            }
            if occupied {
                continue;
            }
            if area > 0.0 {
                consume([face[a], face[b], face[c]])?;
            }
            remaining.remove(i);
            clipped = true;
            break;
        }
        if !clipped {
            return None;
        }
    }
    consume([face[remaining[0]], face[remaining[1]], face[remaining[2]]])
}

fn cross(a: [f64; 2], b: [f64; 2], c: [f64; 2]) -> f64 {
    (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0])
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    fn triangles(text: &str) -> Option<Vec<mesh::Triangle>> {
        let mut result = Vec::new();
        let mut reader = BufReader::with_capacity(19, Cursor::new(text.as_bytes()));
        visit(
            &mut reader,
            text.len() as u64,
            Instant::now() + mesh::TIME_LIMIT,
            &mut |triangle| {
                result.push(triangle);
                Some(())
            },
        )?;
        Some(result)
    }

    #[test]
    fn relative_indices_face_attributes_and_continuations_share_geometry() {
        let vertices = "v 0 0 0\nv 1 0 0\nv 0 1 0\n";
        let expected = triangles(&format!("{vertices}f 1 2 3\n")).unwrap();
        for face in [
            "f -3/-1/1 -2/2/2 -1/3/3",
            "f 1//1 2//1 3//1",
            "f 1 2 \\\n3 # triangle",
            "f 1 2 3 1",
        ] {
            assert_eq!(
                triangles(&format!("{vertices}{face}")),
                Some(expected.clone())
            );
        }
        assert_eq!(expected[0][2], [0., -0., 1.]);
        assert_eq!(
            triangles("v 0 0 0 2\nv 2 0 0 2\nv 0 2 0 2\nf 1 2 3"),
            Some(expected)
        );
    }

    #[test]
    fn concave_faces_preserve_area_and_winding() {
        let vertices = "v 0 0 0\nv 3 0 0\nv 3 1 0\nv 1 1 0\nv 1 3 0\nv 0 3 0\n";
        for (indices, sign) in [("1 2 3 4 5 6", 1.), ("6 5 4 3 2 1", -1.)] {
            let triangles = triangles(&format!("{vertices}f {indices}")).unwrap();
            assert_eq!(triangles.len(), 4);
            let mut area = 0.;
            for triangle in triangles {
                let p = triangle.map(|p| [f64::from(p[0]), f64::from(p[2])]);
                let signed = cross(p[0], p[1], p[2]) * 0.5;
                assert!(signed * sign > 0.);
                area += signed;
                let centroid = [
                    (p[0][0] + p[1][0] + p[2][0]) / 3.,
                    (p[0][1] + p[1][1] + p[2][1]) / 3.,
                ];
                assert!(centroid[0] <= 1. || centroid[1] <= 1.);
            }
            assert!((area - 5. * sign).abs() < 1e-9);
        }
    }

    #[test]
    fn malformed_vertices_indices_and_unbounded_faces_are_refused() {
        for text in [
            "v NaN 0 0",
            "v 0 inf 0",
            "v 1 2",
            "v 1 2 3 0",
            "v 0 0 0\nf 0 1 1",
            "v 0 0 0\nf 1 2 3",
            "v 0 0 0\nf -2 -1 -1",
            "v 0 0 0\nf 1",
            "v 0 0 0\nf 1 1 1",
            "v 0 0 0\\",
            "surf 0 1",
        ] {
            assert!(triangles(text).is_none(), "{text}");
        }
        assert!(
            triangles(&format!(
                "v 0 0 0\nf {}",
                "1 ".repeat(MAX_FACE_VERTICES + 1)
            ))
            .is_none()
        );
        assert!(triangles(&"x".repeat(MAX_LINE_BYTES + 1)).is_none());
        let mut tests = MAX_EAR_TESTS;
        assert!(
            triangulate(
                &[[0., 0., 0.], [1., 0., 0.], [1., 1., 0.], [0., 1., 0.]],
                &mut tests,
                Instant::now() + mesh::TIME_LIMIT,
                &mut |_| Some(())
            )
            .is_none()
        );
    }
}
