//! File checksums, computed in one pass and without a dependency.
//!
//! The four algorithms a properties panel is expected to show are small,
//! frozen and fully specified, so they are implemented here rather than
//! pulled from a crate tree. Correctness does not rest on that claim: every
//! one is checked against the vectors published with its specification, which
//! is what the tests at the bottom of this file do.
//!
//! None of this is used to protect anything. The figures let someone compare
//! a download against what a site advertises, which is why MD5 and SHA-1 are
//! present at all despite being unfit for signatures.

use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};

/// Bytes read between two cancellation checks and two progress reports.
/// Large enough that the overhead disappears, small enough that stopping
/// feels immediate even on a slow disk.
const CHUNK: usize = 256 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Algorithm {
    Md5,
    Sha1,
    Sha256,
    Sha512,
}

impl Algorithm {
    /// Every algorithm, in the order a panel lists them: weakest first, which
    /// is also shortest first.
    pub const ALL: [Self; 4] = [Self::Md5, Self::Sha1, Self::Sha256, Self::Sha512];

    /// Name as a checksum file or a download page writes it.
    pub fn label(self) -> &'static str {
        match self {
            Self::Md5 => "MD5",
            Self::Sha1 => "SHA1",
            Self::Sha256 => "SHA256",
            Self::Sha512 => "SHA512",
        }
    }
}

/// One file read, every requested digest.
///
/// Reading a large file once per algorithm would cost four passes over the
/// disk for the same bytes, so each chunk feeds all of them.
#[derive(Default)]
struct Multi {
    md5: Option<Md5>,
    sha1: Option<Sha1>,
    sha256: Option<Sha256>,
    sha512: Option<Sha512>,
}

impl Multi {
    fn new(algorithms: &[Algorithm]) -> Self {
        let mut m = Self::default();
        for a in algorithms {
            match a {
                Algorithm::Md5 => m.md5 = Some(Md5::new()),
                Algorithm::Sha1 => m.sha1 = Some(Sha1::new()),
                Algorithm::Sha256 => m.sha256 = Some(Sha256::new()),
                Algorithm::Sha512 => m.sha512 = Some(Sha512::new()),
            }
        }
        m
    }

    fn update(&mut self, data: &[u8]) {
        if let Some(h) = self.md5.as_mut() {
            h.update(data);
        }
        if let Some(h) = self.sha1.as_mut() {
            h.update(data);
        }
        if let Some(h) = self.sha256.as_mut() {
            h.update(data);
        }
        if let Some(h) = self.sha512.as_mut() {
            h.update(data);
        }
    }

    fn finish(self) -> Vec<(Algorithm, String)> {
        let mut out = Vec::new();
        if let Some(h) = self.md5 {
            out.push((Algorithm::Md5, hex(&h.finish())));
        }
        if let Some(h) = self.sha1 {
            out.push((Algorithm::Sha1, hex(&h.finish())));
        }
        if let Some(h) = self.sha256 {
            out.push((Algorithm::Sha256, hex(&h.finish())));
        }
        if let Some(h) = self.sha512 {
            out.push((Algorithm::Sha512, hex(&h.finish())));
        }
        out
    }
}

/// Digests of `path`, or `None` when `cancel` was raised before the end.
///
/// `progress` is handed the running byte count so a caller can show how far
/// a large file has got. It is called at most once per chunk, never per byte.
pub fn hash_file(
    path: &Path,
    algorithms: &[Algorithm],
    cancel: &AtomicBool,
    mut progress: impl FnMut(u64),
) -> std::io::Result<Option<Vec<(Algorithm, String)>>> {
    let mut file = File::open(path)?;
    let mut hashers = Multi::new(algorithms);
    let mut buffer = vec![0u8; CHUNK];
    let mut done: u64 = 0;

    loop {
        if cancel.load(Ordering::Relaxed) {
            return Ok(None);
        }
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hashers.update(&buffer[..read]);
        done += read as u64;
        progress(done);
    }
    Ok(Some(hashers.finish()))
}

/// Digests of a slice already in memory, for short inputs and for the tests.
pub fn hash_bytes(data: &[u8], algorithms: &[Algorithm]) -> Vec<(Algorithm, String)> {
    let mut hashers = Multi::new(algorithms);
    hashers.update(data);
    hashers.finish()
}

/// Lowercase hexadecimal, the form every checksum file and download page uses.
fn hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push(DIGITS[(b >> 4) as usize] as char);
        s.push(DIGITS[(b & 0x0f) as usize] as char);
    }
    s
}

/// Whether `candidate` is one of `digests`, comparing without case and
/// ignoring the surrounding blanks a paste usually carries.
///
/// A checksum file line is often `<digest>  <filename>`, so only the first
/// word is considered.
pub fn matches_any(digests: &[(Algorithm, String)], candidate: &str) -> Option<Algorithm> {
    // `split_whitespace` already skips the leading blanks a paste carries.
    let wanted = candidate.split_whitespace().next()?.to_ascii_lowercase();
    if wanted.is_empty() {
        return None;
    }
    digests
        .iter()
        .find(|(_, value)| *value == wanted)
        .map(|(algorithm, _)| *algorithm)
}

// ----- MD5, RFC 1321 -----

/// Per-round addition constants, the integer parts of 2^32 × |sin(i + 1)|.
#[rustfmt::skip]
const MD5_K: [u32; 64] = [
    0xd76aa478, 0xe8c7b756, 0x242070db, 0xc1bdceee,
    0xf57c0faf, 0x4787c62a, 0xa8304613, 0xfd469501,
    0x698098d8, 0x8b44f7af, 0xffff5bb1, 0x895cd7be,
    0x6b901122, 0xfd987193, 0xa679438e, 0x49b40821,
    0xf61e2562, 0xc040b340, 0x265e5a51, 0xe9b6c7aa,
    0xd62f105d, 0x02441453, 0xd8a1e681, 0xe7d3fbc8,
    0x21e1cde6, 0xc33707d6, 0xf4d50d87, 0x455a14ed,
    0xa9e3e905, 0xfcefa3f8, 0x676f02d9, 0x8d2a4c8a,
    0xfffa3942, 0x8771f681, 0x6d9d6122, 0xfde5380c,
    0xa4beea44, 0x4bdecfa9, 0xf6bb4b60, 0xbebfbc70,
    0x289b7ec6, 0xeaa127fa, 0xd4ef3085, 0x04881d05,
    0xd9d4d039, 0xe6db99e5, 0x1fa27cf8, 0xc4ac5665,
    0xf4292244, 0x432aff97, 0xab9423a7, 0xfc93a039,
    0x655b59c3, 0x8f0ccc92, 0xffeff47d, 0x85845dd1,
    0x6fa87e4f, 0xfe2ce6e0, 0xa3014314, 0x4e0811a1,
    0xf7537e82, 0xbd3af235, 0x2ad7d2bb, 0xeb86d391,
];

#[rustfmt::skip]
const MD5_SHIFT: [u32; 64] = [
    7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22,
    5,  9, 14, 20, 5,  9, 14, 20, 5,  9, 14, 20, 5,  9, 14, 20,
    4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23,
    6, 10, 15, 21, 6, 10, 15, 21, 6, 10, 15, 21, 6, 10, 15, 21,
];

struct Md5 {
    state: [u32; 4],
    buffer: [u8; 64],
    buffered: usize,
    length: u64,
}

impl Md5 {
    fn new() -> Self {
        Self {
            state: [0x67452301, 0xefcdab89, 0x98badcfe, 0x10325476],
            buffer: [0; 64],
            buffered: 0,
            length: 0,
        }
    }

    fn update(&mut self, mut data: &[u8]) {
        self.length = self.length.wrapping_add(data.len() as u64);
        // Top up a partial block first, then consume whole blocks in place.
        if self.buffered > 0 {
            let take = (64 - self.buffered).min(data.len());
            self.buffer[self.buffered..self.buffered + take].copy_from_slice(&data[..take]);
            self.buffered += take;
            data = &data[take..];
            // Still a partial block: return now. Falling through would reach
            // the tail below, which assigns rather than appends and would
            // reset the count to the length of what is left, namely nothing.
            if self.buffered < 64 {
                return;
            }
            let block = self.buffer;
            self.compress(&block);
            self.buffered = 0;
        }
        while data.len() >= 64 {
            let (block, rest) = data.split_at(64);
            self.compress(block.try_into().expect("64 bytes"));
            data = rest;
        }
        self.buffer[..data.len()].copy_from_slice(data);
        self.buffered = data.len();
    }

    fn finish(mut self) -> [u8; 16] {
        // 0x80, then zeros up to 56 bytes, then the bit length. MD5 writes
        // that length little-endian, unlike the SHA family.
        let bits = self.length.wrapping_mul(8);
        self.update(&[0x80]);
        while self.buffered != 56 {
            self.update(&[0]);
        }
        let mut tail = [0u8; 8];
        tail.copy_from_slice(&bits.to_le_bytes());
        self.update(&tail);
        debug_assert_eq!(self.buffered, 0);

        let mut out = [0u8; 16];
        for (i, word) in self.state.iter().enumerate() {
            out[i * 4..i * 4 + 4].copy_from_slice(&word.to_le_bytes());
        }
        out
    }

    fn compress(&mut self, block: &[u8; 64]) {
        let mut m = [0u32; 16];
        for (i, word) in m.iter_mut().enumerate() {
            *word = u32::from_le_bytes(block[i * 4..i * 4 + 4].try_into().expect("4 bytes"));
        }
        let [mut a, mut b, mut c, mut d] = self.state;
        for i in 0..64 {
            let (f, g) = match i / 16 {
                0 => ((b & c) | (!b & d), i),
                1 => ((d & b) | (!d & c), (5 * i + 1) % 16),
                2 => (b ^ c ^ d, (3 * i + 5) % 16),
                _ => (c ^ (b | !d), (7 * i) % 16),
            };
            let tmp = d;
            d = c;
            c = b;
            let sum = a.wrapping_add(f).wrapping_add(MD5_K[i]).wrapping_add(m[g]);
            b = b.wrapping_add(sum.rotate_left(MD5_SHIFT[i]));
            a = tmp;
        }
        self.state[0] = self.state[0].wrapping_add(a);
        self.state[1] = self.state[1].wrapping_add(b);
        self.state[2] = self.state[2].wrapping_add(c);
        self.state[3] = self.state[3].wrapping_add(d);
    }
}

// ----- SHA-1, RFC 3174 -----

struct Sha1 {
    state: [u32; 5],
    buffer: [u8; 64],
    buffered: usize,
    length: u64,
}

impl Sha1 {
    fn new() -> Self {
        Self {
            state: [0x67452301, 0xefcdab89, 0x98badcfe, 0x10325476, 0xc3d2e1f0],
            buffer: [0; 64],
            buffered: 0,
            length: 0,
        }
    }

    fn update(&mut self, mut data: &[u8]) {
        self.length = self.length.wrapping_add(data.len() as u64);
        if self.buffered > 0 {
            let take = (64 - self.buffered).min(data.len());
            self.buffer[self.buffered..self.buffered + take].copy_from_slice(&data[..take]);
            self.buffered += take;
            data = &data[take..];
            // Still a partial block: return now. Falling through would reach
            // the tail below, which assigns rather than appends and would
            // reset the count to the length of what is left, namely nothing.
            if self.buffered < 64 {
                return;
            }
            let block = self.buffer;
            self.compress(&block);
            self.buffered = 0;
        }
        while data.len() >= 64 {
            let (block, rest) = data.split_at(64);
            self.compress(block.try_into().expect("64 bytes"));
            data = rest;
        }
        self.buffer[..data.len()].copy_from_slice(data);
        self.buffered = data.len();
    }

    fn finish(mut self) -> [u8; 20] {
        let bits = self.length.wrapping_mul(8);
        self.update(&[0x80]);
        while self.buffered != 56 {
            self.update(&[0]);
        }
        let tail = bits.to_be_bytes();
        self.update(&tail);
        debug_assert_eq!(self.buffered, 0);

        let mut out = [0u8; 20];
        for (i, word) in self.state.iter().enumerate() {
            out[i * 4..i * 4 + 4].copy_from_slice(&word.to_be_bytes());
        }
        out
    }

    fn compress(&mut self, block: &[u8; 64]) {
        let mut w = [0u32; 80];
        for (i, word) in w.iter_mut().take(16).enumerate() {
            *word = u32::from_be_bytes(block[i * 4..i * 4 + 4].try_into().expect("4 bytes"));
        }
        for i in 16..80 {
            w[i] = (w[i - 3] ^ w[i - 8] ^ w[i - 14] ^ w[i - 16]).rotate_left(1);
        }
        let [mut a, mut b, mut c, mut d, mut e] = self.state;
        for (i, word) in w.iter().enumerate() {
            let (f, k) = match i / 20 {
                0 => ((b & c) | (!b & d), 0x5a827999),
                1 => (b ^ c ^ d, 0x6ed9eba1),
                2 => ((b & c) | (b & d) | (c & d), 0x8f1bbcdc),
                _ => (b ^ c ^ d, 0xca62c1d6),
            };
            let tmp = a
                .rotate_left(5)
                .wrapping_add(f)
                .wrapping_add(e)
                .wrapping_add(k)
                .wrapping_add(*word);
            e = d;
            d = c;
            c = b.rotate_left(30);
            b = a;
            a = tmp;
        }
        self.state[0] = self.state[0].wrapping_add(a);
        self.state[1] = self.state[1].wrapping_add(b);
        self.state[2] = self.state[2].wrapping_add(c);
        self.state[3] = self.state[3].wrapping_add(d);
        self.state[4] = self.state[4].wrapping_add(e);
    }
}

// ----- SHA-256 and SHA-512, FIPS 180-4 -----

/// Fractional parts of the cube roots of the first sixty-four primes.
#[rustfmt::skip]
const SHA256_K: [u32; 64] = [
    0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
    0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
    0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
    0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
    0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
    0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
    0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
    0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
];

struct Sha256 {
    state: [u32; 8],
    buffer: [u8; 64],
    buffered: usize,
    length: u64,
}

impl Sha256 {
    fn new() -> Self {
        Self {
            state: [
                0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
                0x5be0cd19,
            ],
            buffer: [0; 64],
            buffered: 0,
            length: 0,
        }
    }

    fn update(&mut self, mut data: &[u8]) {
        self.length = self.length.wrapping_add(data.len() as u64);
        if self.buffered > 0 {
            let take = (64 - self.buffered).min(data.len());
            self.buffer[self.buffered..self.buffered + take].copy_from_slice(&data[..take]);
            self.buffered += take;
            data = &data[take..];
            // Still a partial block: return now. Falling through would reach
            // the tail below, which assigns rather than appends and would
            // reset the count to the length of what is left, namely nothing.
            if self.buffered < 64 {
                return;
            }
            let block = self.buffer;
            self.compress(&block);
            self.buffered = 0;
        }
        while data.len() >= 64 {
            let (block, rest) = data.split_at(64);
            self.compress(block.try_into().expect("64 bytes"));
            data = rest;
        }
        self.buffer[..data.len()].copy_from_slice(data);
        self.buffered = data.len();
    }

    fn finish(mut self) -> [u8; 32] {
        let bits = self.length.wrapping_mul(8);
        self.update(&[0x80]);
        while self.buffered != 56 {
            self.update(&[0]);
        }
        let tail = bits.to_be_bytes();
        self.update(&tail);
        debug_assert_eq!(self.buffered, 0);

        let mut out = [0u8; 32];
        for (i, word) in self.state.iter().enumerate() {
            out[i * 4..i * 4 + 4].copy_from_slice(&word.to_be_bytes());
        }
        out
    }

    fn compress(&mut self, block: &[u8; 64]) {
        let mut w = [0u32; 64];
        for (i, word) in w.iter_mut().take(16).enumerate() {
            *word = u32::from_be_bytes(block[i * 4..i * 4 + 4].try_into().expect("4 bytes"));
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16]
                .wrapping_add(s0)
                .wrapping_add(w[i - 7])
                .wrapping_add(s1);
        }
        let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut h] = self.state;
        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ (!e & g);
            let t1 = h
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(SHA256_K[i])
                .wrapping_add(w[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let t2 = s0.wrapping_add(maj);
            h = g;
            g = f;
            f = e;
            e = d.wrapping_add(t1);
            d = c;
            c = b;
            b = a;
            a = t1.wrapping_add(t2);
        }
        for (slot, value) in self.state.iter_mut().zip([a, b, c, d, e, f, g, h]) {
            *slot = slot.wrapping_add(value);
        }
    }
}

/// Fractional parts of the cube roots of the first eighty primes.
#[rustfmt::skip]
const SHA512_K: [u64; 80] = [
    0x428a2f98d728ae22, 0x7137449123ef65cd, 0xb5c0fbcfec4d3b2f, 0xe9b5dba58189dbbc,
    0x3956c25bf348b538, 0x59f111f1b605d019, 0x923f82a4af194f9b, 0xab1c5ed5da6d8118,
    0xd807aa98a3030242, 0x12835b0145706fbe, 0x243185be4ee4b28c, 0x550c7dc3d5ffb4e2,
    0x72be5d74f27b896f, 0x80deb1fe3b1696b1, 0x9bdc06a725c71235, 0xc19bf174cf692694,
    0xe49b69c19ef14ad2, 0xefbe4786384f25e3, 0x0fc19dc68b8cd5b5, 0x240ca1cc77ac9c65,
    0x2de92c6f592b0275, 0x4a7484aa6ea6e483, 0x5cb0a9dcbd41fbd4, 0x76f988da831153b5,
    0x983e5152ee66dfab, 0xa831c66d2db43210, 0xb00327c898fb213f, 0xbf597fc7beef0ee4,
    0xc6e00bf33da88fc2, 0xd5a79147930aa725, 0x06ca6351e003826f, 0x142929670a0e6e70,
    0x27b70a8546d22ffc, 0x2e1b21385c26c926, 0x4d2c6dfc5ac42aed, 0x53380d139d95b3df,
    0x650a73548baf63de, 0x766a0abb3c77b2a8, 0x81c2c92e47edaee6, 0x92722c851482353b,
    0xa2bfe8a14cf10364, 0xa81a664bbc423001, 0xc24b8b70d0f89791, 0xc76c51a30654be30,
    0xd192e819d6ef5218, 0xd69906245565a910, 0xf40e35855771202a, 0x106aa07032bbd1b8,
    0x19a4c116b8d2d0c8, 0x1e376c085141ab53, 0x2748774cdf8eeb99, 0x34b0bcb5e19b48a8,
    0x391c0cb3c5c95a63, 0x4ed8aa4ae3418acb, 0x5b9cca4f7763e373, 0x682e6ff3d6b2b8a3,
    0x748f82ee5defb2fc, 0x78a5636f43172f60, 0x84c87814a1f0ab72, 0x8cc702081a6439ec,
    0x90befffa23631e28, 0xa4506cebde82bde9, 0xbef9a3f7b2c67915, 0xc67178f2e372532b,
    0xca273eceea26619c, 0xd186b8c721c0c207, 0xeada7dd6cde0eb1e, 0xf57d4f7fee6ed178,
    0x06f067aa72176fba, 0x0a637dc5a2c898a6, 0x113f9804bef90dae, 0x1b710b35131c471b,
    0x28db77f523047d84, 0x32caab7b40c72493, 0x3c9ebe0a15c9bebc, 0x431d67c49c100d4c,
    0x4cc5d4becb3e42b6, 0x597f299cfc657e2a, 0x5fcb6fab3ad6faec, 0x6c44198c4a475817,
];

struct Sha512 {
    state: [u64; 8],
    buffer: [u8; 128],
    buffered: usize,
    length: u64,
}

impl Sha512 {
    fn new() -> Self {
        Self {
            state: [
                0x6a09e667f3bcc908,
                0xbb67ae8584caa73b,
                0x3c6ef372fe94f82b,
                0xa54ff53a5f1d36f1,
                0x510e527fade682d1,
                0x9b05688c2b3e6c1f,
                0x1f83d9abfb41bd6b,
                0x5be0cd19137e2179,
            ],
            buffer: [0; 128],
            buffered: 0,
            length: 0,
        }
    }

    fn update(&mut self, mut data: &[u8]) {
        self.length = self.length.wrapping_add(data.len() as u64);
        if self.buffered > 0 {
            let take = (128 - self.buffered).min(data.len());
            self.buffer[self.buffered..self.buffered + take].copy_from_slice(&data[..take]);
            self.buffered += take;
            data = &data[take..];
            // Still a partial block: return now. Falling through would reach
            // the tail below, which assigns rather than appends and would
            // reset the count to the length of what is left, namely nothing.
            if self.buffered < 128 {
                return;
            }
            let block = self.buffer;
            self.compress(&block);
            self.buffered = 0;
        }
        while data.len() >= 128 {
            let (block, rest) = data.split_at(128);
            self.compress(block.try_into().expect("128 bytes"));
            data = rest;
        }
        self.buffer[..data.len()].copy_from_slice(data);
        self.buffered = data.len();
    }

    fn finish(mut self) -> [u8; 64] {
        let bits = self.length.wrapping_mul(8);
        self.update(&[0x80]);
        while self.buffered != 112 {
            self.update(&[0]);
        }
        // The length field is 128 bits wide. A file long enough to need the
        // upper half cannot exist, so those eight bytes are always zero.
        let mut tail = [0u8; 16];
        tail[8..].copy_from_slice(&bits.to_be_bytes());
        self.update(&tail);
        debug_assert_eq!(self.buffered, 0);

        let mut out = [0u8; 64];
        for (i, word) in self.state.iter().enumerate() {
            out[i * 8..i * 8 + 8].copy_from_slice(&word.to_be_bytes());
        }
        out
    }

    fn compress(&mut self, block: &[u8; 128]) {
        let mut w = [0u64; 80];
        for (i, word) in w.iter_mut().take(16).enumerate() {
            *word = u64::from_be_bytes(block[i * 8..i * 8 + 8].try_into().expect("8 bytes"));
        }
        for i in 16..80 {
            let s0 = w[i - 15].rotate_right(1) ^ w[i - 15].rotate_right(8) ^ (w[i - 15] >> 7);
            let s1 = w[i - 2].rotate_right(19) ^ w[i - 2].rotate_right(61) ^ (w[i - 2] >> 6);
            w[i] = w[i - 16]
                .wrapping_add(s0)
                .wrapping_add(w[i - 7])
                .wrapping_add(s1);
        }
        let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut h] = self.state;
        for i in 0..80 {
            let s1 = e.rotate_right(14) ^ e.rotate_right(18) ^ e.rotate_right(41);
            let ch = (e & f) ^ (!e & g);
            let t1 = h
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(SHA512_K[i])
                .wrapping_add(w[i]);
            let s0 = a.rotate_right(28) ^ a.rotate_right(34) ^ a.rotate_right(39);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let t2 = s0.wrapping_add(maj);
            h = g;
            g = f;
            f = e;
            e = d.wrapping_add(t1);
            d = c;
            c = b;
            b = a;
            a = t1.wrapping_add(t2);
        }
        for (slot, value) in self.state.iter_mut().zip([a, b, c, d, e, f, g, h]) {
            *slot = slot.wrapping_add(value);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    /// Every expected value below is published with the algorithm's own
    /// specification: RFC 1321 for MD5, RFC 3174 for SHA-1, FIPS 180-4 for
    /// the SHA-2 pair. They are the reason these implementations can be
    /// trusted without being taken on faith.
    fn digest(algorithm: Algorithm, data: &[u8]) -> String {
        hash_bytes(data, &[algorithm])
            .into_iter()
            .next()
            .expect("one digest")
            .1
    }

    /// 56 bytes: one byte too many for a single block once the padding and
    /// the length are added, so it exercises the second block.
    const TWO_BLOCKS: &str = "abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq";

    /// 112 bytes, the SHA-512 equivalent of the case above.
    const TWO_BLOCKS_512: &str = "abcdefghbcdefghicdefghijdefghijkefghijklfghijklmghijklmnhijklmnoijklmnopjklmnopqklmnopqrlmnopqrsmnopqrstnopqrstu";

    #[test]
    fn md5_matches_the_rfc_1321_vectors() {
        let a = Algorithm::Md5;
        assert_eq!(digest(a, b""), "d41d8cd98f00b204e9800998ecf8427e");
        assert_eq!(digest(a, b"a"), "0cc175b9c0f1b6a831c399e269772661");
        assert_eq!(digest(a, b"abc"), "900150983cd24fb0d6963f7d28e17f72");
        assert_eq!(
            digest(a, b"message digest"),
            "f96b697d7cb7938d525a2f31aaf161d0"
        );
        assert_eq!(
            digest(a, b"abcdefghijklmnopqrstuvwxyz"),
            "c3fcd3d76192e4007dfb496cca67e13b"
        );
        assert_eq!(
            digest(
                a,
                b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789"
            ),
            "d174ab98d277d9f5a5611c2c9f419d9f"
        );
        assert_eq!(
            digest(
                a,
                b"12345678901234567890123456789012345678901234567890123456789012345678901234567890"
            ),
            "57edf4a22be3c955ac49da2e2107b67a"
        );
    }

    #[test]
    fn sha1_matches_the_rfc_3174_vectors() {
        let a = Algorithm::Sha1;
        assert_eq!(digest(a, b""), "da39a3ee5e6b4b0d3255bfef95601890afd80709");
        assert_eq!(
            digest(a, b"abc"),
            "a9993e364706816aba3e25717850c26c9cd0d89d"
        );
        assert_eq!(
            digest(a, TWO_BLOCKS.as_bytes()),
            "84983e441c3bd26ebaae4aa1f95129e5e54670f1"
        );
    }

    #[test]
    fn sha256_matches_the_fips_180_4_vectors() {
        let a = Algorithm::Sha256;
        assert_eq!(
            digest(a, b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert_eq!(
            digest(a, b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        assert_eq!(
            digest(a, TWO_BLOCKS.as_bytes()),
            "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1"
        );
    }

    #[test]
    fn sha512_matches_the_fips_180_4_vectors() {
        let a = Algorithm::Sha512;
        assert_eq!(
            digest(a, b""),
            "cf83e1357eefb8bdf1542850d66d8007d620e4050b5715dc83f4a921d36ce9ce\
             47d0d13c5d85f2b0ff8318d2877eec2f63b931bd47417a81a538327af927da3e"
        );
        assert_eq!(
            digest(a, b"abc"),
            "ddaf35a193617abacc417349ae20413112e6fa4e89a97ea20a9eeee64b55d39a\
             2192992a274fc1a836ba3c23a3feebbd454d4423643ce80e2a9ac94fa54ca49f"
        );
        assert_eq!(
            digest(a, TWO_BLOCKS_512.as_bytes()),
            "8e959b75dae313da8cf4f72814fc143f8f7779c6eb9f7fa17299aeadb6889018\
             501d289e4900f7e4331b99dec4b5433ac7d329eeb6dd26545e96e55b874be909"
        );
    }

    #[test]
    fn a_long_input_exercises_the_block_loop_and_the_length_field() {
        // One million letters, the long vector published with each algorithm.
        // A fault in the block loop or in the encoded bit length shows here
        // and nowhere in the short cases above.
        let data = vec![b'a'; 1_000_000];
        assert_eq!(
            digest(Algorithm::Md5, &data),
            "7707d6ae4e027c70eea2a935c2296f21"
        );
        assert_eq!(
            digest(Algorithm::Sha1, &data),
            "34aa973cd4c4daa4f61eeb2bdbad27316534016f"
        );
        assert_eq!(
            digest(Algorithm::Sha256, &data),
            "cdc76e5c9914fb9281a1c7e284d73e67f1809a48a497200e046d39ccc7112cd0"
        );
        assert_eq!(
            digest(Algorithm::Sha512, &data),
            "e718483d0ce769644e2e42c7bc15b4638e1f98b13b2044285632a803afa973eb\
             de0ff244877ea60a4cb0432ce577c31beb009c5c2c49aa2e4eadb217ad8cc09b"
        );
    }

    #[test]
    fn feeding_one_byte_at_a_time_gives_the_same_answer() {
        // The buffering is where a streaming digest goes wrong, and a file is
        // read in chunks whose boundaries fall anywhere.
        let data: Vec<u8> = (0u8..=255).cycle().take(1000).collect();
        for algorithm in Algorithm::ALL {
            let whole = digest(algorithm, &data);
            let mut multi = Multi::new(&[algorithm]);
            for byte in &data {
                multi.update(std::slice::from_ref(byte));
            }
            let piecemeal = multi.finish().into_iter().next().expect("one digest").1;
            assert_eq!(whole, piecemeal, "{}", algorithm.label());
        }
    }

    #[test]
    fn one_pass_over_a_file_answers_every_algorithm() {
        let dir = std::env::temp_dir().join(format!("favnyr-checksum-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("scratch directory");
        let path = dir.join("my_file.bin");
        std::fs::File::create(&path)
            .expect("create")
            .write_all(b"abc")
            .expect("write");

        let mut seen = 0u64;
        let digests = hash_file(&path, &Algorithm::ALL, &AtomicBool::new(false), |n| {
            seen = n
        })
        .expect("read")
        .expect("not cancelled");

        assert_eq!(digests.len(), 4);
        assert_eq!(seen, 3, "progress must report the bytes read");
        assert_eq!(digests[0].1, "900150983cd24fb0d6963f7d28e17f72");
        assert_eq!(digests[2].1, digest(Algorithm::Sha256, b"abc"));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_raised_flag_stops_before_reading() {
        let dir = std::env::temp_dir().join(format!("favnyr-cancel-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("scratch directory");
        let path = dir.join("my_file.bin");
        std::fs::File::create(&path)
            .expect("create")
            .write_all(b"abc")
            .expect("write");

        let outcome =
            hash_file(&path, &Algorithm::ALL, &AtomicBool::new(true), |_| {}).expect("read");
        assert!(outcome.is_none(), "cancellation must yield no digest");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_pasted_checksum_is_recognised_whatever_its_shape() {
        let digests = hash_bytes(b"abc", &Algorithm::ALL);
        let sha256 = "BA7816BF8F01CFEA414140DE5DAE2223B00361A396177A9CB410FF61F20015AD";

        // Upper case, as several download pages publish it.
        assert_eq!(matches_any(&digests, sha256), Some(Algorithm::Sha256));
        // Surrounded by blanks, as a paste usually arrives.
        assert_eq!(
            matches_any(&digests, &format!("  {sha256}\n")),
            Some(Algorithm::Sha256)
        );
        // A whole line of a checksum file, digest then file name.
        assert_eq!(
            matches_any(&digests, &format!("{sha256}  my_file.bin")),
            Some(Algorithm::Sha256)
        );
        assert_eq!(matches_any(&digests, "not a digest"), None);
        assert_eq!(matches_any(&digests, "   "), None);
    }
}
