//! Video duration probes and representative frame extraction with local tools.

use std::path::Path;
use std::process::Command;

use super::tools::{TOOL_TIMEOUT, no_console, run_bounded};
use super::{Thumbnail, from_encoded};

/// Position of the preview in the timeline: **30% of the duration**. Many
/// videos open on a black screen / logo; a later moment gives a
/// recognizable thumbnail, and the fraction scales just as well from a
/// 10s clip to a 2h movie.
const VIDEO_THUMB_FRACTION: f64 = 0.30;
/// Number of frames analyzed by ffmpeg's `thumbnail` filter to pick the
/// most representative one (the one furthest from the batch average → avoids
/// a uniform black screen). Used ONLY when the duration is unknown: with
/// a known duration, seeking to 30% is enough and cheaper (no decoding
/// of N extra frames).
const VIDEO_THUMB_ANALYZE_FRAMES: u32 = 30;
/// Input seek (seconds) for the "unknown duration" fallback , a rare case: neither
/// the ISO-BMFF header nor `ffprobe` yielded a duration. Far enough in to skip past an
/// intro/logo; if the video is shorter, the seek fails and the caller retries
/// without seeking (cf. [`from_video`]).
const VIDEO_THUMB_SKIP_SECS: f64 = 5.0;

/// Seek position (seconds) for a given duration. `None` (unknown duration)
/// → flat intro skip.
fn video_seek_offset(duration_secs: Option<f64>) -> f64 {
    match duration_secs {
        Some(d) if d.is_finite() && d > 0.0 => d * VIDEO_THUMB_FRACTION,
        _ => VIDEO_THUMB_SKIP_SECS,
    }
}

/// Duration of a video in seconds, at the best possible cost:
/// 1. DIRECT read of the ISO-BMFF header (`moov`/`mvhd`) for
///    MP4/MOV/M4V/3GP containers , **~0.1 ms**, no process spawned;
/// 2. otherwise (Matroska/WebM/AVI…), probes `ffprobe` , correct but ~200 ms,
///    dominated purely by the binary's startup.
///
/// `None` if neither path succeeds → the caller falls back to the
/// "intro skip + representative frame" heuristic.
fn video_duration_secs(path: &Path) -> Option<f64> {
    mp4_duration_secs(path).or_else(|| ffprobe_duration_secs(path))
}

/// Duration read from the **ISO-BMFF** header: the `mvhd` box (direct child of `moov`),
/// which carries `timescale` (units/second) and `duration` (in units). We only read
/// box headers , a few dozen bytes, without decoding the media.
/// `None` if the file isn't ISO-BMFF, if `mvhd` is absent, or if the duration
/// is zero (the case for fragmented MP4s, where it lives in the fragments).
fn mp4_duration_secs(path: &Path) -> Option<f64> {
    use std::io::{Read, Seek, SeekFrom};

    /// Reads a box header at the current position → `(total_size, type)`.
    /// `None` at end of stream or if the size is inconsistent.
    fn read_box_header(f: &mut std::fs::File, limit: u64) -> Option<(u64, [u8; 4])> {
        let start = f.stream_position().ok()?;
        let mut hdr = [0u8; 8];
        f.read_exact(&mut hdr).ok()?;
        let mut size = u32::from_be_bytes([hdr[0], hdr[1], hdr[2], hdr[3]]) as u64;
        let typ = [hdr[4], hdr[5], hdr[6], hdr[7]];
        if size == 1 {
            // 64-bit extended size (large files).
            let mut ext = [0u8; 8];
            f.read_exact(&mut ext).ok()?;
            size = u64::from_be_bytes(ext);
        } else if size == 0 {
            // The box extends to the end of the container.
            size = limit.checked_sub(start)?;
        }
        // A box smaller than its header, or that overflows, is invalid:
        // we reject it rather than looping forever on a malformed file.
        if size < 8 || start.checked_add(size)? > limit {
            return None;
        }
        Some((size, typ))
    }

    let mut f = std::fs::File::open(path).ok()?;
    let file_len = f.metadata().ok()?.len();

    // Root level: we look for `moov` (it can follow a large `mdat`).
    let mut cursor = 0u64;
    while cursor < file_len {
        f.seek(SeekFrom::Start(cursor)).ok()?;
        let (size, typ) = read_box_header(&mut f, file_len)?;
        if &typ == b"moov" {
            // Inside `moov`, `mvhd` is a DIRECT child (no need to descend
            // into trak/mdia): we scan just this one level.
            let moov_end = cursor + size;
            let mut inner = f.stream_position().ok()?; // right after the header
            while inner < moov_end {
                f.seek(SeekFrom::Start(inner)).ok()?;
                let (isize_, ityp) = read_box_header(&mut f, moov_end)?;
                if &ityp == b"mvhd" {
                    let mut ver = [0u8; 4]; // version (1) + flags (3)
                    f.read_exact(&mut ver).ok()?;
                    let (timescale, duration) = if ver[0] == 1 {
                        let mut buf = [0u8; 28]; // creation(8) mtime(8) ts(4) duration(8)
                        f.read_exact(&mut buf).ok()?;
                        let ts = u32::from_be_bytes([buf[16], buf[17], buf[18], buf[19]]);
                        let du = u64::from_be_bytes([
                            buf[20], buf[21], buf[22], buf[23], buf[24], buf[25], buf[26], buf[27],
                        ]);
                        (ts, du)
                    } else {
                        let mut buf = [0u8; 16]; // creation(4) mtime(4) ts(4) duration(4)
                        f.read_exact(&mut buf).ok()?;
                        let ts = u32::from_be_bytes([buf[8], buf[9], buf[10], buf[11]]);
                        let du = u32::from_be_bytes([buf[12], buf[13], buf[14], buf[15]]) as u64;
                        (ts, du)
                    };
                    if timescale == 0 || duration == 0 {
                        return None; // fragmented MP4 / atypical header
                    }
                    let secs = duration as f64 / timescale as f64;
                    return secs.is_finite().then_some(secs);
                }
                inner = inner.checked_add(isize_)?;
            }
            return None; // `moov` without a usable `mvhd`
        }
        cursor = cursor.checked_add(size)?;
    }
    None
}

/// Duration via the **`ffprobe`** CLI (shipped with `ffmpeg`): reads METADATA
/// only (no decoding). Fallback for non-ISO-BMFF containers. `None` if `ffprobe`
/// is missing, fails, or doesn't expose a duration.
fn ffprobe_duration_secs(path: &Path) -> Option<f64> {
    let mut cmd = Command::new("ffprobe");
    cmd.args([
        "-v",
        "error",
        "-show_entries",
        "format=duration",
        "-of",
        "default=noprint_wrappers=1:nokey=1",
    ])
    .arg(path);
    no_console(&mut cmd);
    let stdout = run_bounded(cmd, TOOL_TIMEOUT)?;
    let d: f64 = String::from_utf8_lossy(&stdout).trim().parse().ok()?;
    (d.is_finite() && d > 0.0).then_some(d)
}

/// Thumbnail of a video file via the `ffmpeg` CLI, in **a single decoding
/// process**: `scale` bounds the large side to `max_px` INSIDE ffmpeg (no
/// full-resolution PNG re-decoded on the Rust side).
///
/// The targeted frame is at **30% of the timeline** ([`VIDEO_THUMB_FRACTION`]) as soon as
/// the duration is known , free for MP4s (`mvhd` header). Without a duration,
/// we fall back to the "intro skip + `thumbnail` filter" heuristic (representative
/// frame among N). In both cases, if the seek goes past the end (a very short
/// video → empty output), we retry without seeking. `None` if `ffmpeg` is
/// not found or fails everywhere.
pub fn from_video(path: &Path, max_px: u32) -> Option<Thumbnail> {
    let duration = video_duration_secs(path);
    let seek = video_seek_offset(duration);
    // Known duration → seeking to 30% is enough; otherwise we let ffmpeg pick a
    // representative frame (costs decoding N frames, hence the targeted use).
    let analyze = duration.is_none();
    from_video_frame(path, max_px, Some(seek), analyze)
        .or_else(|| from_video_frame(path, max_px, None, true))
}

fn from_video_frame(
    path: &Path,
    max_px: u32,
    seek_secs: Option<f64>,
    analyze: bool,
) -> Option<Thumbnail> {
    // `scale=…decrease` bounds the large side to max_px without ever upscaling
    // (min(max_px, dim)); `thumbnail=n=N` (optional) picks a
    // representative frame among N.
    let scale =
        format!("scale='min({max_px},iw)':'min({max_px},ih)':force_original_aspect_ratio=decrease");
    let vf = if analyze {
        format!("thumbnail=n={VIDEO_THUMB_ANALYZE_FRAMES},{scale}")
    } else {
        scale
    };
    let mut cmd = Command::new("ffmpeg");
    cmd.args(["-v", "error"]);
    // -ss BEFORE -i = fast input seek (keyframe ≤ target) → doesn't decode
    // from the start. Omitted on fallback (video shorter than the target).
    if let Some(secs) = seek_secs {
        cmd.args(["-ss", &format!("{secs:.3}")]);
    }
    cmd.arg("-i").arg(path).args([
        "-frames:v",
        "1",
        "-an",
        "-vf",
        &vf,
        "-f",
        "image2pipe",
        "-vcodec",
        "png",
        "pipe:1",
    ]);
    no_console(&mut cmd);
    let stdout = run_bounded(cmd, TOOL_TIMEOUT)?;
    if stdout.is_empty() {
        return None;
    }
    // The PNG ffmpeg just produced goes through the same bounded decode as any
    // other encoded bytes: it is derived from an untrusted video.
    from_encoded(&stdout, max_px)
}

#[cfg(test)]
mod tests {
    use super::super::test_support::tempdir;
    use super::*;

    #[test]
    fn video_seek_targets_thirty_percent_with_safe_fallback() {
        // Known duration → 30% of the timeline.
        assert_eq!(video_seek_offset(Some(100.0)), 30.0);
        assert_eq!(video_seek_offset(Some(10.0)), 3.0);
        // Unknown / invalid duration → flat 5s intro skip (value
        // hardcoded in the assertion: an accidental change must fail the test).
        assert_eq!(VIDEO_THUMB_SKIP_SECS, 5.0);
        assert_eq!(video_seek_offset(None), 5.0);
        assert_eq!(video_seek_offset(Some(0.0)), 5.0);
        assert_eq!(video_seek_offset(Some(-5.0)), 5.0);
        assert_eq!(video_seek_offset(Some(f64::NAN)), 5.0);
        assert_eq!(video_seek_offset(Some(f64::INFINITY)), 5.0);
    }

    /// Builds a minimal MP4 `ftyp` + `moov`/`mvhd` (version 0) carrying
    /// `timescale`/`duration`, to validate box traversal without depending
    /// on a committed binary file or on ffmpeg.
    fn write_fake_mp4(path: &std::path::Path, timescale: u32, duration: u32) {
        fn boxed(typ: &[u8; 4], payload: &[u8]) -> Vec<u8> {
            let mut v = ((payload.len() + 8) as u32).to_be_bytes().to_vec();
            v.extend_from_slice(typ);
            v.extend_from_slice(payload);
            v
        }
        let mut mvhd = vec![0u8; 4]; // version 0 + flags
        mvhd.extend_from_slice(&0u32.to_be_bytes()); // creation
        mvhd.extend_from_slice(&0u32.to_be_bytes()); // modification
        mvhd.extend_from_slice(&timescale.to_be_bytes());
        mvhd.extend_from_slice(&duration.to_be_bytes());
        mvhd.extend_from_slice(&[0u8; 80]); // rest of mvhd (ignored)

        let mut file = boxed(b"ftyp", b"isom\0\0\x02\0isomiso2");
        // Large `mdat` BEFORE `moov`: traversal must skip it cleanly.
        file.extend_from_slice(&boxed(b"mdat", &vec![0u8; 4096]));
        file.extend_from_slice(&boxed(b"moov", &boxed(b"mvhd", &mvhd)));
        std::fs::write(path, file).unwrap();
    }

    #[test]
    fn mp4_duration_read_from_header_without_spawning_ffprobe() {
        let dir = tempdir();
        // 19.064s at scale 1000 (real-world case of a common MP4).
        let p = dir.join("clip.mp4");
        write_fake_mp4(&p, 1000, 19_064);
        let d = mp4_duration_secs(&p).expect("duration readable from the header");
        assert!((d - 19.064).abs() < 1e-6, "duration read = {d}");
        // The targeted seek is indeed 30% of this duration.
        assert!((video_seek_offset(Some(d)) - 5.7192).abs() < 1e-4);

        // Fragmented MP4 (zero duration in mvhd) → None (ffprobe fallback on the caller's side).
        let frag = dir.join("frag.mp4");
        write_fake_mp4(&frag, 1000, 0);
        assert_eq!(mp4_duration_secs(&frag), None);

        // Non-ISO-BMFF file → None, without panicking or looping.
        let txt = dir.join("not.mp4");
        std::fs::write(&txt, b"definitely not an mp4 file").unwrap();
        assert_eq!(mp4_duration_secs(&txt), None);

        std::fs::remove_dir_all(&dir).ok();
    }
}
