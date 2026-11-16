//! Own FLAC encoder (A-013, MAS-7): STREAMINFO with MD5, fixed blocks of 4096 frames, fixed predictors of order
//! 0–4 (or constant/verbatim) chosen per subframe, partitioned Rice coding, stereo decorrelation chosen per frame,
//! CRC-8 frame headers and CRC-16 frame footers. Output is a pure function of the input (MAS-8).

use std::fs;
use std::path::Path;

use crate::ExportError;

const BLOCK: usize = 4096;
const MAX_PARTITION_ORDER: u32 = 8;

/// Writes `samples` (interleaved, already quantised to `bits`, i.e. within ±2^(bits−1)) as a FLAC file.
/// `channels` is 1 or 2; `bits` is 16 or 24; `rate` any rate up to 655350 Hz.
pub fn encode_flac(path: &Path, samples: &[i32], channels: u16, rate: u32, bits: u32) -> crate::Result<()> {
    if !(1..=2).contains(&channels) {
        return Err(ExportError::Unsupported(format!("FLAC with {channels} channels")));
    }
    if bits != 16 && bits != 24 {
        return Err(ExportError::Unsupported(format!("FLAC with {bits} bits per sample")));
    }
    if rate == 0 || rate > 655_350 {
        return Err(ExportError::Unsupported(format!("FLAC sample rate {rate}")));
    }
    let ch = usize::from(channels);
    if !samples.len().is_multiple_of(ch) {
        return Err(ExportError::Unsupported("sample count is not a multiple of the channel count".into()));
    }
    let limit = 1i32 << (bits - 1);
    if samples.iter().any(|&s| s < -limit || s >= limit) {
        return Err(ExportError::Unsupported(format!("sample out of the {bits}-bit range")));
    }
    let total = samples.len() / ch;

    let mut md5 = Md5::new();
    let width = (bits / 8) as usize;
    let mut buf = Vec::with_capacity(4096 * width);
    for chunk in samples.chunks(4096) {
        buf.clear();
        for &s in chunk {
            buf.extend_from_slice(&s.to_le_bytes()[..width]);
        }
        md5.update(&buf);
    }
    let digest = md5.finalize();

    let mut frames = Vec::new();
    let (mut min_frame, mut max_frame) = (u32::MAX, 0u32);
    let mut left = Vec::with_capacity(BLOCK);
    let mut right = Vec::with_capacity(BLOCK);
    let mut start = 0usize;
    let mut number = 0u32;
    while start < total {
        let n = BLOCK.min(total - start);
        left.clear();
        right.clear();
        for i in 0..n {
            left.push(samples[(start + i) * ch]);
            if ch == 2 {
                right.push(samples[(start + i) * ch + 1]);
            }
        }
        let before = frames.len();
        encode_frame(&mut frames, number, &left, (ch == 2).then_some(&right[..]), bits);
        let size = (frames.len() - before) as u32;
        min_frame = min_frame.min(size);
        max_frame = max_frame.max(size);
        start += n;
        number += 1;
    }
    if number == 0 {
        min_frame = 0;
    }

    let blocks_min = if total < BLOCK { total.max(16) } else { BLOCK } as u32;
    let blocks_max = if total < BLOCK { total.max(16) } else { BLOCK } as u32;
    let mut out = Vec::with_capacity(frames.len() + 42);
    out.extend_from_slice(b"fLaC");
    out.extend_from_slice(&[0x80, 0, 0, 34]);
    let mut si = BitWriter::new();
    si.bits(u64::from(blocks_min), 16);
    si.bits(u64::from(blocks_max), 16);
    si.bits(u64::from(min_frame), 24);
    si.bits(u64::from(max_frame), 24);
    si.bits(u64::from(rate), 20);
    si.bits(u64::from(channels - 1), 3);
    si.bits(u64::from(bits - 1), 5);
    si.bits(total as u64, 36);
    out.extend_from_slice(&si.bytes);
    out.extend_from_slice(&digest);
    out.extend_from_slice(&frames);

    let mut tmp = path.as_os_str().to_owned();
    tmp.push(".tmp");
    let tmp = std::path::PathBuf::from(tmp);
    let result = fs::write(&tmp, &out).and_then(|()| fs::rename(&tmp, path));
    if result.is_err() {
        let _ = fs::remove_file(&tmp);
    }
    Ok(result?)
}

/// Encodes one block (`right` is `Some` for stereo) and appends the complete frame to `out`.
fn encode_frame(out: &mut Vec<u8>, number: u32, left: &[i32], right: Option<&[i32]>, bits: u32) {
    let n = left.len();
    let start = out.len();
    // Header.
    out.push(0xFF);
    out.push(0xF8);
    let bs_code: u8 = match n {
        BLOCK => 12,
        1..=256 => 6,
        _ => 7,
    };
    // Channel assignment is patched below; keep its index.
    out.push(bs_code << 4);
    let chan_at = out.len();
    out.push(0);
    let bps_code: u8 = if bits == 16 { 0b100 } else { 0b110 };

    // Subframes.
    let mut body = BitWriter::new();
    let chan_code: u8;
    match right {
        None => {
            chan_code = 0;
            let p = plan(left, bits);
            write_subframe(&mut body, &p, left, bits);
        }
        Some(right) => {
            let side: Vec<i32> = left.iter().zip(right).map(|(l, r)| l - r).collect();
            let mid: Vec<i32> = left.iter().zip(right).map(|(l, r)| (l + r) >> 1).collect();
            let pl = plan(left, bits);
            let pr = plan(right, bits);
            let ps = plan(&side, bits + 1);
            let pm = plan(&mid, bits);
            let options =
                [(1u8, pl.bits + pr.bits), (8, pl.bits + ps.bits), (9, ps.bits + pr.bits), (10, pm.bits + ps.bits)];
            let mut best = options[0];
            for o in &options[1..] {
                if o.1 < best.1 {
                    best = *o;
                }
            }
            chan_code = best.0;
            match chan_code {
                1 => {
                    write_subframe(&mut body, &pl, left, bits);
                    write_subframe(&mut body, &pr, right, bits);
                }
                8 => {
                    write_subframe(&mut body, &pl, left, bits);
                    write_subframe(&mut body, &ps, &side, bits + 1);
                }
                9 => {
                    write_subframe(&mut body, &ps, &side, bits + 1);
                    write_subframe(&mut body, &pr, right, bits);
                }
                _ => {
                    write_subframe(&mut body, &pm, &mid, bits);
                    write_subframe(&mut body, &ps, &side, bits + 1);
                }
            }
        }
    }
    out[chan_at] = (chan_code << 4) | (bps_code << 1);
    utf8_number(out, number);
    if bs_code == 6 {
        out.push((n - 1) as u8);
    } else if bs_code == 7 {
        out.extend_from_slice(&((n - 1) as u16).to_be_bytes());
    }
    out.push(crc8(&out[start..]));
    body.pad();
    out.extend_from_slice(&body.bytes);
    let crc = crc16(&out[start..]);
    out.extend_from_slice(&crc.to_be_bytes());
}

fn utf8_number(out: &mut Vec<u8>, v: u32) {
    if v < 0x80 {
        out.push(v as u8);
        return;
    }
    let extra = if v < 0x800 {
        1
    } else if v < 0x1_0000 {
        2
    } else if v < 0x20_0000 {
        3
    } else if v < 0x400_0000 {
        4
    } else {
        5
    };
    let lead = (0xFF00u16 >> (extra + 1)) as u8;
    out.push(lead | (v >> (6 * extra)) as u8);
    for i in (0..extra).rev() {
        out.push(0x80 | ((v >> (6 * i)) & 0x3F) as u8);
    }
}

fn crc8(data: &[u8]) -> u8 {
    let mut crc = 0u8;
    for &b in data {
        crc ^= b;
        for _ in 0..8 {
            crc = if crc & 0x80 != 0 { (crc << 1) ^ 0x07 } else { crc << 1 };
        }
    }
    crc
}

fn crc16(data: &[u8]) -> u16 {
    let mut crc = 0u16;
    for &b in data {
        crc ^= u16::from(b) << 8;
        for _ in 0..8 {
            crc = if crc & 0x8000 != 0 { (crc << 1) ^ 0x8005 } else { crc << 1 };
        }
    }
    crc
}

/// Most-significant-bit-first bit writer.
struct BitWriter {
    bytes: Vec<u8>,
    acc: u64,
    n: u32,
}

impl BitWriter {
    fn new() -> Self {
        Self { bytes: Vec::new(), acc: 0, n: 0 }
    }

    /// Writes the low `count` bits of `value`.
    fn bits(&mut self, value: u64, count: u32) {
        let mut count = count;
        while count > 0 {
            let take = count.min(32);
            count -= take;
            let part = (value >> count) & ((1u64 << take) - 1);
            self.acc = (self.acc << take) | part;
            self.n += take;
            while self.n >= 8 {
                self.n -= 8;
                self.bytes.push((self.acc >> self.n) as u8);
            }
            self.acc &= (1u64 << self.n) - 1;
        }
    }

    fn signed(&mut self, v: i64, count: u32) {
        self.bits((v as u64) & ((1u64 << count) - 1), count);
    }

    fn unary_zeros_then_one(&mut self, zeros: u64) {
        let mut z = zeros;
        while z >= 32 {
            self.bits(0, 32);
            z -= 32;
        }
        self.bits(1, z as u32 + 1);
    }

    fn pad(&mut self) {
        if self.n > 0 {
            self.bits(0, 8 - self.n);
        }
    }
}

#[derive(Clone)]
enum Kind {
    Constant,
    Verbatim,
    Fixed { order: usize, method: u8, porder: u32, params: Vec<u32> },
}

#[derive(Clone)]
struct Plan {
    bits: u64,
    kind: Kind,
}

fn residual(x: &[i32], order: usize) -> Vec<i64> {
    let g = |i: usize, k: usize| i64::from(x[i - k]);
    (order..x.len())
        .map(|i| {
            let v = i64::from(x[i]);
            match order {
                0 => v,
                1 => v - g(i, 1),
                2 => v - 2 * g(i, 1) + g(i, 2),
                3 => v - 3 * g(i, 1) + 3 * g(i, 2) - g(i, 3),
                _ => v - 4 * g(i, 1) + 6 * g(i, 2) - 4 * g(i, 3) + g(i, 4),
            }
        })
        .collect()
}

fn zigzag(r: i64) -> u64 {
    ((r << 1) ^ (r >> 63)) as u64
}

/// Best Rice parameter for `u` within `0..=max_k`, as `(k, bits)` (bits exclude the parameter field).
fn best_rice(u: &[u64], max_k: u32) -> (u32, u64) {
    let sum: u64 = u.iter().sum();
    let count = u.len() as u64;
    let est = if count == 0 || sum < count { 0 } else { 63 - (sum / count).leading_zeros() }.min(max_k);
    let mut best = (0u32, u64::MAX);
    for k in est.saturating_sub(1)..=(est + 1).min(max_k) {
        let bits: u64 = u.iter().map(|&v| v >> k).sum::<u64>() + count * u64::from(k + 1);
        if bits < best.1 {
            best = (k, bits);
        }
    }
    best
}

/// Chooses the cheapest coding of one channel signal.
fn plan(x: &[i32], bps: u32) -> Plan {
    let n = x.len();
    let verbatim = Plan { bits: 8 + n as u64 * u64::from(bps), kind: Kind::Verbatim };
    if x.iter().all(|&v| v == x[0]) {
        return Plan { bits: 8 + u64::from(bps), kind: Kind::Constant };
    }
    // Pick the predictor order by the sum of absolute residuals, then search partitions only for it.
    let mut best_order = 0;
    let mut best_res = Vec::new();
    let mut best_abs = u64::MAX;
    for order in 0..=4usize.min(n.saturating_sub(1)) {
        let r = residual(x, order);
        let abs: u64 = r.iter().map(|v| v.unsigned_abs()).sum();
        if abs < best_abs {
            best_abs = abs;
            best_order = order;
            best_res = r;
        }
    }
    let u: Vec<u64> = best_res.iter().map(|&r| zigzag(r)).collect();
    let mut best: Option<Plan> = None;
    for porder in 0..=MAX_PARTITION_ORDER {
        let parts = 1usize << porder;
        if !n.is_multiple_of(parts) || n / parts <= best_order {
            break;
        }
        let size = n / parts;
        for method in 0..2u8 {
            let (max_k, field) = if method == 0 { (14, 4u64) } else { (30, 5u64) };
            let mut params = Vec::with_capacity(parts);
            let mut bits = 8 + best_order as u64 * u64::from(bps) + 6;
            let mut at = 0usize;
            for p in 0..parts {
                let len = if p == 0 { size - best_order } else { size };
                let (k, b) = best_rice(&u[at..at + len], max_k);
                at += len;
                params.push(k);
                bits += field + b;
            }
            if best.as_ref().is_none_or(|b| bits < b.bits) {
                best = Some(Plan { bits, kind: Kind::Fixed { order: best_order, method, porder, params } });
            }
        }
    }
    match best {
        Some(b) if b.bits < verbatim.bits => b,
        _ => verbatim,
    }
}

fn write_subframe(w: &mut BitWriter, plan: &Plan, x: &[i32], bps: u32) {
    match &plan.kind {
        Kind::Constant => {
            w.bits(0, 8);
            w.signed(i64::from(x[0]), bps);
        }
        Kind::Verbatim => {
            w.bits(0b0000_0010, 8);
            for &v in x {
                w.signed(i64::from(v), bps);
            }
        }
        Kind::Fixed { order, method, porder, params } => {
            w.bits(0b0001_0000 | ((*order as u64) << 1), 8);
            for &v in &x[..*order] {
                w.signed(i64::from(v), bps);
            }
            w.bits(u64::from(*method), 2);
            w.bits(u64::from(*porder), 4);
            let field = if *method == 0 { 4 } else { 5 };
            let res = residual(x, *order);
            let size = x.len() >> porder;
            let mut at = 0usize;
            for (p, &k) in params.iter().enumerate() {
                let len = if p == 0 { size - order } else { size };
                w.bits(u64::from(k), field);
                for &r in &res[at..at + len] {
                    let u = zigzag(r);
                    w.unary_zeros_then_one(u >> k);
                    if k > 0 {
                        w.bits(u & ((1u64 << k) - 1), k);
                    }
                }
                at += len;
            }
        }
    }
}

/// MD5 (RFC 1321), used for the STREAMINFO signature of the unencoded audio.
struct Md5 {
    state: [u32; 4],
    buf: [u8; 64],
    fill: usize,
    total: u64,
}

const MD5_S: [u32; 64] = [
    7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 5, 9, 14, 20, 5, 9, 14, 20, 5, 9, 14, 20, 5, 9, 14, 20,
    4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 6, 10, 15, 21, 6, 10, 15, 21, 6, 10, 15, 21, 6, 10, 15,
    21,
];

impl Md5 {
    fn new() -> Self {
        Self { state: [0x6745_2301, 0xefcd_ab89, 0x98ba_dcfe, 0x1032_5476], buf: [0; 64], fill: 0, total: 0 }
    }

    fn block(&mut self, chunk: &[u8; 64]) {
        let mut m = [0u32; 16];
        for (i, w) in m.iter_mut().enumerate() {
            *w = u32::from_le_bytes([chunk[4 * i], chunk[4 * i + 1], chunk[4 * i + 2], chunk[4 * i + 3]]);
        }
        let [mut a, mut b, mut c, mut d] = self.state;
        for (i, &shift) in MD5_S.iter().enumerate() {
            let (f, g) = match i / 16 {
                0 => ((b & c) | (!b & d), i),
                1 => ((d & b) | (!d & c), (5 * i + 1) % 16),
                2 => (b ^ c ^ d, (3 * i + 5) % 16),
                _ => (c ^ (b | !d), (7 * i) % 16),
            };
            let k = ((i as f64 + 1.0).sin().abs() * 4_294_967_296.0) as u32;
            let sum = a.wrapping_add(f).wrapping_add(k).wrapping_add(m[g]);
            a = d;
            d = c;
            c = b;
            b = b.wrapping_add(sum.rotate_left(shift));
        }
        for (s, v) in self.state.iter_mut().zip([a, b, c, d]) {
            *s = s.wrapping_add(v);
        }
    }

    fn update(&mut self, data: &[u8]) {
        self.total += data.len() as u64;
        let mut data = data;
        while !data.is_empty() {
            let take = (64 - self.fill).min(data.len());
            self.buf[self.fill..self.fill + take].copy_from_slice(&data[..take]);
            self.fill += take;
            data = &data[take..];
            if self.fill == 64 {
                let chunk = self.buf;
                self.block(&chunk);
                self.fill = 0;
            }
        }
    }

    fn finalize(mut self) -> [u8; 16] {
        let bit_len = self.total.wrapping_mul(8);
        self.update(&[0x80]);
        while self.fill != 56 {
            self.update(&[0]);
        }
        self.update(&bit_len.to_le_bytes());
        let mut out = [0u8; 16];
        for (i, s) in self.state.iter().enumerate() {
            out[4 * i..4 * i + 4].copy_from_slice(&s.to_le_bytes());
        }
        out
    }
}

/// MD5 of `data` (exposed for tests that check the STREAMINFO signature).
#[doc(hidden)]
pub fn md5(data: &[u8]) -> [u8; 16] {
    let mut m = Md5::new();
    m.update(data);
    m.finalize()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hex(d: [u8; 16]) -> String {
        d.iter().map(|b| format!("{b:02x}")).collect()
    }

    /// req: MAS-7
    #[test]
    fn flac_md5_known_vectors() {
        assert_eq!(hex(md5(b"")), "d41d8cd98f00b204e9800998ecf8427e");
        assert_eq!(hex(md5(b"abc")), "900150983cd24fb0d6963f7d28e17f72");
        assert_eq!(
            hex(md5(b"12345678901234567890123456789012345678901234567890123456789012345678901234567890")),
            "57edf4a22be3c955ac49da2e2107b67a"
        );
    }

    /// req: MAS-7
    #[test]
    fn flac_utf8_frame_numbers() {
        let mut v = Vec::new();
        utf8_number(&mut v, 0x7F);
        utf8_number(&mut v, 0x80);
        utf8_number(&mut v, 0x800);
        utf8_number(&mut v, 0x1_0000);
        assert_eq!(v, [0x7F, 0xC2, 0x80, 0xE0, 0xA0, 0x80, 0xF0, 0x90, 0x80, 0x80]);
    }
}
