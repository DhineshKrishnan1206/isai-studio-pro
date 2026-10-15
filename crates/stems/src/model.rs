//! The Spleeter 4-stem model: turning Deezer's release into a compact weights
//! file, and loading it.
//!
//! Deezer publishes the model as a TensorFlow checkpoint inside a .tar.gz.
//! The checkpoint's index is a sorted table (LevelDB's SSTable format) of
//! tensor entries pointing into the data file; this reads it directly, so no
//! TensorFlow is needed. Only the network's weights are kept, as 16-bit floats.

use std::collections::HashMap;
use std::io::Read;

use sha2::{Digest, Sha256};

/// Deezer's release of the model (MIT licence) and its published SHA-256.
pub const MODEL_URL: &str = "https://github.com/deezer/spleeter/releases/download/v1.4.0/4stems.tar.gz";
pub const MODEL_SHA256: &str = "3adb4a50ad4eb18c7c4d65fcf4cf2367a07d48408a5eb7d03cd20067429dfaa8";
/// Its size, for showing download progress.
pub const MODEL_BYTES: u64 = 146_308_147;
pub const LICENSE: &str = "Spleeter 4-stem model (c) Deezer Research, MIT License, https://github.com/deezer/spleeter";

const MAGIC: &[u8; 10] = b"ISAISTEMS2";

fn varint(buf: &[u8], mut pos: usize) -> Option<(u64, usize)> {
    let (mut out, mut shift) = (0u64, 0);
    loop {
        let b = *buf.get(pos)?;
        pos += 1;
        out |= u64::from(b & 0x7F) << shift;
        if b < 0x80 {
            return Some((out, pos));
        }
        shift += 7;
        if shift > 63 {
            return None;
        }
    }
}

/// Key/value pairs of one uncompressed SSTable block.
fn block_entries(data: &[u8]) -> Option<Vec<(Vec<u8>, Vec<u8>)>> {
    let n = u32::from_le_bytes(data.get(data.len().checked_sub(4)?..)?.try_into().ok()?) as usize;
    let end = data.len().checked_sub(4 + 4 * n)?;
    let (mut pos, mut key, mut out) = (0usize, Vec::new(), Vec::new());
    while pos < end {
        let (shared, p) = varint(data, pos)?;
        let (unshared, p) = varint(data, p)?;
        let (vlen, p) = varint(data, p)?;
        key.truncate(shared as usize);
        key.extend_from_slice(data.get(p..p + unshared as usize)?);
        let v = p + unshared as usize;
        out.push((key.clone(), data.get(v..v + vlen as usize)?.to_vec()));
        pos = v + vlen as usize;
    }
    Some(out)
}

fn read_block(buf: &[u8], offset: u64, size: u64) -> Option<&[u8]> {
    let (o, s) = (offset as usize, size as usize);
    if *buf.get(o + s)? != 0 {
        return None; // compressed blocks: not used by TensorFlow checkpoints
    }
    buf.get(o..o + s)
}

/// A float tensor in a checkpoint: name, offset and size in the data file, shape.
type Entry = (String, u64, u64, Vec<usize>);

/// Every float tensor in a checkpoint index.
fn index_entries(index: &[u8]) -> Option<Vec<Entry>> {
    let footer = index.get(index.len().checked_sub(48)?..)?;
    let (_, p) = varint(footer, 0)?;
    let (_, p) = varint(footer, p)?;
    let (off, p) = varint(footer, p)?;
    let (size, _) = varint(footer, p)?;
    let mut out = Vec::new();
    for (_, handle) in block_entries(read_block(index, off, size)?)? {
        let (o, p) = varint(&handle, 0)?;
        let (s, _) = varint(&handle, p)?;
        for (key, value) in block_entries(read_block(index, o, s)?)? {
            let name = String::from_utf8(key).ok()?;
            if name.is_empty() {
                continue;
            }
            // BundleEntryProto: 1 dtype, 2 shape { 2 dim { 1 size } }, 4 offset, 5 size.
            let (mut pos, mut dtype, mut shape, mut offset, mut len) = (0, 0, Vec::new(), 0, 0);
            while pos < value.len() {
                let (tag, p) = varint(&value, pos)?;
                pos = p;
                match (tag >> 3, tag & 7) {
                    (f, 0) => {
                        let (v, p) = varint(&value, pos)?;
                        pos = p;
                        match f {
                            1 => dtype = v,
                            4 => offset = v,
                            5 => len = v,
                            _ => {}
                        }
                    }
                    (f, 2) => {
                        let (l, p) = varint(&value, pos)?;
                        let sub = value.get(p..p + l as usize)?;
                        pos = p + l as usize;
                        if f == 2 {
                            let mut sp = 0;
                            while sp < sub.len() {
                                let (t, q) = varint(sub, sp)?;
                                let (l2, q) = varint(sub, q)?;
                                let dim = sub.get(q..q + l2 as usize)?;
                                sp = q + l2 as usize;
                                if t >> 3 == 2 {
                                    let mut dp = 0;
                                    while dp < dim.len() {
                                        let (t2, r) = varint(dim, dp)?;
                                        let (v, r) = varint(dim, r)?;
                                        dp = r;
                                        if t2 >> 3 == 1 {
                                            shape.push(v as usize);
                                        }
                                    }
                                }
                            }
                        }
                    }
                    (_, 5) => pos += 4,
                    (_, 1) => pos += 8,
                    _ => return None,
                }
            }
            if dtype == 1 && !name.contains("Adam") && !name.ends_with("_power") {
                out.push((name, offset, len, shape));
            }
        }
    }
    Some(out)
}

fn f32_to_f16(v: f32) -> u16 {
    let x = v.to_bits();
    let sign = ((x >> 16) & 0x8000) as u16;
    let exp = ((x >> 23) & 0xFF) as i32 - 127 + 15;
    let mant = x & 0x7F_FFFF;
    if exp <= 0 {
        if exp < -10 {
            return sign;
        }
        let m = (mant | 0x80_0000) >> (1 - exp);
        return sign | ((m + 0x1000) >> 13) as u16;
    }
    if exp >= 31 {
        return sign | 0x7C00;
    }
    // Round to nearest.
    let h = sign as u32 | ((exp as u32) << 10) | (mant >> 13);
    (h + ((mant >> 12) & 1)) as u16
}

fn f16_to_f32(h: u16) -> f32 {
    let sign = u32::from(h & 0x8000) << 16;
    let exp = u32::from((h >> 10) & 0x1F);
    let mant = u32::from(h & 0x3FF);
    let bits = match exp {
        0 if mant == 0 => sign,
        0 => {
            // Subnormal: normalise.
            let mut e = 127 - 15 + 1;
            let mut m = mant;
            while m & 0x400 == 0 {
                m <<= 1;
                e -= 1;
            }
            sign | (e << 23) | ((m & 0x3FF) << 13)
        }
        31 => sign | 0x7F80_0000 | (mant << 13),
        _ => sign | ((exp + 127 - 15) << 23) | (mant << 13),
    };
    f32::from_bits(bits)
}

/// Checks Deezer's .tar.gz and turns it into Isai's weights file.
pub fn convert_release(tar_gz: &[u8]) -> Result<Vec<u8>, String> {
    let digest = Sha256::digest(tar_gz);
    let hex: String = digest.iter().map(|b| format!("{b:02x}")).collect();
    if hex != MODEL_SHA256 {
        return Err("the downloaded model doesn't match Deezer's published checksum".into());
    }
    let mut archive = tar::Archive::new(flate2::read::GzDecoder::new(tar_gz));
    let (mut index, mut data) = (None, None);
    for entry in archive.entries().map_err(|e| e.to_string())? {
        let mut entry = entry.map_err(|e| e.to_string())?;
        let name = entry.path().map_err(|e| e.to_string())?.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        let mut bytes = Vec::new();
        if name == "model.index" || name.starts_with("model.data-") {
            entry.read_to_end(&mut bytes).map_err(|e| e.to_string())?;
            if name == "model.index" {
                index = Some(bytes);
            } else {
                data = Some(bytes);
            }
        }
    }
    let (index, data) = (index.ok_or("the archive has no model index")?, data.ok_or("the archive has no model data")?);
    let entries = index_entries(&index).ok_or("couldn't read the model index")?;
    let mut tensors = Vec::new();
    let mut blob: Vec<u8> = Vec::new();
    for (name, offset, len, shape) in entries {
        let raw = data.get(offset as usize..(offset + len) as usize).ok_or("the model data is cut short")?;
        tensors.push(serde_json::json!({ "name": name, "shape": shape, "offset": blob.len() / 2 }));
        for chunk in raw.as_chunks::<4>().0 {
            let v = f32::from_le_bytes(*chunk);
            blob.extend_from_slice(&f32_to_f16(v).to_le_bytes());
        }
    }
    let header = serde_json::json!({ "tensors": tensors, "license": LICENSE }).to_string();
    let mut out = Vec::with_capacity(MAGIC.len() + 4 + header.len() + blob.len());
    out.extend_from_slice(MAGIC);
    out.extend_from_slice(&(header.len() as u32).to_le_bytes());
    out.extend_from_slice(header.as_bytes());
    out.extend_from_slice(&blob);
    Ok(out)
}

/// The loaded weights, by tensor name.
pub struct Weights {
    tensors: HashMap<String, (Vec<usize>, Vec<f32>)>,
}

impl Weights {
    pub fn load(file: &[u8]) -> Result<Self, String> {
        if file.get(..MAGIC.len()) != Some(&MAGIC[..]) {
            return Err("not an Isai stem model file".into());
        }
        let hlen = u32::from_le_bytes(file.get(10..14).ok_or("short file")?.try_into().map_err(|_| "short file")?) as usize;
        let header: serde_json::Value = serde_json::from_slice(file.get(14..14 + hlen).ok_or("short file")?).map_err(|e| e.to_string())?;
        let blob = &file[14 + hlen..];
        let mut tensors = HashMap::new();
        for t in header["tensors"].as_array().ok_or("bad header")? {
            let name = t["name"].as_str().ok_or("bad header")?.to_owned();
            let shape: Vec<usize> = t["shape"].as_array().ok_or("bad header")?.iter().filter_map(|d| d.as_u64().map(|v| v as usize)).collect();
            let count: usize = shape.iter().product();
            let at = t["offset"].as_u64().ok_or("bad header")? as usize * 2;
            let raw = blob.get(at..at + count * 2).ok_or("the model file is cut short")?;
            let values = raw.as_chunks::<2>().0.iter().map(|c| f16_to_f32(u16::from_le_bytes(*c))).collect();
            tensors.insert(name, (shape, values));
        }
        Ok(Self { tensors })
    }

    pub fn get(&self, name: &str) -> Result<&(Vec<usize>, Vec<f32>), String> {
        self.tensors.get(name).ok_or_else(|| format!("the model has no {name}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn half_floats_round_trip_closely() {
        for v in [0.0f32, 1.0, -2.5, 0.0001234, 65000.0, -1e-6, 1.234_567] {
            let back = f16_to_f32(f32_to_f16(v));
            assert!((back - v).abs() <= v.abs() * 1e-3 + 1e-7, "{v} -> {back}");
        }
    }
}
