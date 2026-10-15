//! Canonical visually-imperceptible PNG carrier.
//!
//! The visible raster is rendered from the exact SVG and coalesced by at most one
//! 8-bit code value per RGB channel before lossless PNG compression. The complete
//! source capsule is stored separately in a private ancillary PNG chunk instead of
//! being written into noisy RGB low bits. This makes the visible change bounded,
//! deterministic, and effectively imperceptible while preserving byte-exact source
//! recovery (and keyed source confidentiality) from the original file.
use crate::{archive, compression, decode_svg_names_with_key, Capsule, MagicKey};
use serde::{Deserialize, Serialize};
use std::io::Read;

const MAGIC_CHUNK: &[u8; 4] = b"ciMg"; // ancillary/private/reserved-valid/safe-to-copy
const FORMAT_MAGIC: &[u8; 8] = b"CIMVIS01";
const BROTLI_MAGIC: &[u8; 8] = b"CIMPB001";
const DOMAIN: &[u8] = b"perceptual-png/v1";
const MAX_ARCHIVE: usize = 128_000_000;
const MAX_PIXELS: usize = 64_000_000;

#[derive(Serialize, Deserialize)]
struct StoredPerceptualCapsule {
    version: u32,
    capsule: Capsule,
}

fn packed(capsule: &Capsule, key: Option<&MagicKey>) -> Result<Vec<u8>, String> {
    let mut capsule = capsule.clone();
    capsule.snapshot.canonicalize()?;
    if capsule.version != 1 {
        return Err("Unsupported capsule version".into());
    }
    if capsule.svg.contains("data-magic-key='1'") && key.is_none() {
        return Err("A magic key is required to export this keyed artwork".into());
    }
    if capsule.svg.contains("data-magic-key='1'") && capsule.svg.contains("data-name-sigil='1'") {
        // Refuse to protect source with a key different from the one used for contours.
        decode_svg_names_with_key(&capsule.svg, key)?;
    }
    // Shared glyph definitions reduce both the vector payload and the authenticated
    // source carrier without changing any visible outline or transform.
    capsule.svg = compression::compact_svg(&capsule.svg)?;
    let raw = serde_json::to_vec(&StoredPerceptualCapsule {
        version: 1,
        capsule,
    })
    .map_err(|e| e.to_string())?;
    if raw.len() > MAX_ARCHIVE {
        return Err("Archive too large".into());
    }
    let mut compressed = BROTLI_MAGIC.to_vec();
    let mut encoder = brotli::CompressorReader::new(raw.as_slice(), 4096, 9, 22);
    encoder
        .take((MAX_ARCHIVE + 1) as u64)
        .read_to_end(&mut compressed)
        .map_err(|e| e.to_string())?;
    if compressed.len() > MAX_ARCHIVE {
        return Err("Compressed archive exceeds budget".into());
    }
    let mut out = FORMAT_MAGIC.to_vec();
    match key {
        Some(key) => {
            out.push(1);
            out.extend(key.seal(DOMAIN, &compressed)?);
        }
        None => {
            out.push(0);
            out.extend(compressed);
        }
    }
    if out.len() > MAX_ARCHIVE {
        return Err("Protected archive exceeds budget".into());
    }
    Ok(out)
}

fn unpack(bytes: &[u8], key: Option<&MagicKey>) -> Result<Capsule, String> {
    let payload = bytes
        .strip_prefix(FORMAT_MAGIC)
        .ok_or("Invalid visually-imperceptible source capsule")?;
    let (&flag, body) = payload
        .split_first()
        .ok_or("Truncated visually-imperceptible source capsule")?;
    let compressed = match flag {
        0 => body.to_vec(),
        1 => key
            .ok_or("This image requires its original magic key")?
            .open(DOMAIN, body)?,
        _ => return Err("Unsupported visually-imperceptible source capsule".into()),
    };
    let data = compressed
        .strip_prefix(BROTLI_MAGIC)
        .ok_or("Invalid visually-imperceptible compression stream")?;
    let mut raw = Vec::new();
    brotli::Decompressor::new(data, 4096)
        .take((MAX_ARCHIVE + 1) as u64)
        .read_to_end(&mut raw)
        .map_err(|_| "Corrupt visually-imperceptible source capsule")?;
    if raw.len() > MAX_ARCHIVE {
        return Err("Decoded archive exceeds budget".into());
    }
    let mut stored: StoredPerceptualCapsule =
        serde_json::from_slice(&raw).map_err(|_| "Invalid visually-imperceptible archive")?;
    if stored.version != 1 || stored.capsule.version != 1 {
        return Err("Unsupported visually-imperceptible archive version".into());
    }
    stored.capsule.snapshot.canonicalize()?;
    if stored.capsule.svg.contains("data-magic-key='1'") {
        decode_svg_names_with_key(&stored.capsule.svg, key)?;
    }
    Ok(stored.capsule)
}

fn chunk_bytes(kind: &[u8; 4], data: &[u8]) -> Result<Vec<u8>, String> {
    let len = u32::try_from(data.len()).map_err(|_| "PNG source capsule exceeds chunk limit")?;
    let mut out = Vec::with_capacity(data.len() + 12);
    out.extend_from_slice(&len.to_be_bytes());
    out.extend_from_slice(kind);
    out.extend_from_slice(data);
    let mut crc_data = Vec::with_capacity(4 + data.len());
    crc_data.extend_from_slice(kind);
    crc_data.extend_from_slice(data);
    out.extend_from_slice(&archive::crc32(&crc_data).to_be_bytes());
    Ok(out)
}

fn insert_magic_chunk(png: &[u8], payload: &[u8]) -> Result<Vec<u8>, String> {
    if !png.starts_with(b"\x89PNG\r\n\x1a\n") {
        return Err("Invalid PNG image".into());
    }
    let mut pos = 8usize;
    while pos + 12 <= png.len() {
        let len = u32::from_be_bytes(png[pos..pos + 4].try_into().unwrap()) as usize;
        let end = pos
            .checked_add(12)
            .and_then(|v| v.checked_add(len))
            .ok_or("PNG chunk length overflow")?;
        if end > png.len() {
            return Err("Truncated PNG image".into());
        }
        let kind = &png[pos + 4..pos + 8];
        if kind == MAGIC_CHUNK {
            return Err("PNG already contains a Code Is Magic source capsule".into());
        }
        if kind == b"IEND" {
            if len != 0 || end != png.len() {
                return Err("Invalid PNG end chunk".into());
            }
            let chunk = chunk_bytes(MAGIC_CHUNK, payload)?;
            let mut out = Vec::with_capacity(png.len() + chunk.len());
            out.extend_from_slice(&png[..pos]);
            out.extend_from_slice(&chunk);
            out.extend_from_slice(&png[pos..]);
            return Ok(out);
        }
        pos = end;
    }
    Err("PNG has no end chunk".into())
}

fn magic_chunk(png: &[u8]) -> Result<Option<Vec<u8>>, String> {
    if !png.starts_with(b"\x89PNG\r\n\x1a\n") {
        return Ok(None);
    }
    let mut found = None;
    let mut pos = 8usize;
    while pos + 12 <= png.len() {
        let len = u32::from_be_bytes(png[pos..pos + 4].try_into().unwrap()) as usize;
        let end = pos
            .checked_add(12)
            .and_then(|v| v.checked_add(len))
            .ok_or("PNG chunk length overflow")?;
        if end > png.len() {
            return Err("Truncated PNG image".into());
        }
        let kind: &[u8; 4] = png[pos + 4..pos + 8].try_into().unwrap();
        if kind == MAGIC_CHUNK {
            if found.is_some() {
                return Err("PNG has multiple Code Is Magic source capsules".into());
            }
            if len > MAX_ARCHIVE {
                return Err("PNG source capsule exceeds budget".into());
            }
            let data = &png[pos + 8..pos + 8 + len];
            let expected = u32::from_be_bytes(png[pos + 8 + len..end].try_into().unwrap());
            let mut crc_data = Vec::with_capacity(4 + len);
            crc_data.extend_from_slice(kind);
            crc_data.extend_from_slice(data);
            if archive::crc32(&crc_data) != expected {
                return Err("PNG source capsule is damaged".into());
            }
            found = Some(data.to_vec());
        }
        if kind == b"IEND" {
            return Ok(found);
        }
        pos = end;
    }
    Err("Truncated PNG image".into())
}

pub(crate) fn preserve_magic_chunk(original: &[u8], optimized: Vec<u8>) -> Result<Vec<u8>, String> {
    match magic_chunk(original)? {
        Some(payload) => insert_magic_chunk(&optimized, &payload),
        None => Ok(optimized),
    }
}

/// Encode the one canonical transmutation format.
///
/// The visible RGB error is mathematically bounded to <= 1/255 per channel.
/// The PNG encoder is then fully lossless; only this one-code-value coalescing is
/// perceptual. Source bytes remain exact in the private ancillary chunk.
pub fn encode_png(capsule: &Capsule) -> Result<Vec<u8>, String> {
    encode_png_with_key(capsule, None)
}

pub fn encode_png_with_key(capsule: &Capsule, key: Option<&MagicKey>) -> Result<Vec<u8>, String> {
    let payload = packed(capsule, key)?;
    let options = resvg::usvg::Options::default();
    let tree = resvg::usvg::Tree::from_str(&capsule.svg, &options)
        .map_err(|_| "Invalid vector artwork")?;
    let size = tree.size();
    // Preserve the previous canonical raster budget. The payload no longer needs
    // pixel capacity, but using the same budget avoids a hidden resolution change.
    let carrier_equivalent = (payload.len() + 20) * 8;
    let needed = carrier_equivalent.div_ceil(9);
    let aspect = (size.width() as f64 / size.height() as f64).clamp(0.1, 10.0);
    let pixels = needed.max((size.width() as usize * size.height() as usize).min(16_000_000));
    if pixels > MAX_PIXELS {
        return Err("Image capacity exceeds 64 million pixels".into());
    }
    let width = ((pixels as f64 * aspect).sqrt().ceil() as u32).max(64);
    let height = (pixels.div_ceil(width as usize) as u32).max(64);
    if width as usize * height as usize > MAX_PIXELS {
        return Err("Image dimensions exceed capacity".into());
    }
    let mut pixmap =
        resvg::tiny_skia::Pixmap::new(width, height).ok_or("Could not allocate PNG")?;
    resvg::render(
        &tree,
        resvg::tiny_skia::Transform::from_scale(
            width as f32 / size.width(),
            height as f32 / size.height(),
        ),
        &mut pixmap.as_mut(),
    );
    for pixel in pixmap.data_mut().as_chunks_mut::<4>().0 {
        pixel[3] = 255;
        // Drop exactly one least-significant code bit. This is deliberately much
        // more conservative than JPEG quantization: channel error is never > 1.
        pixel[0] &= 0xfe;
        pixel[1] &= 0xfe;
        pixel[2] &= 0xfe;
    }
    let png = compression::encode_pixels(pixmap.data(), png::Info::with_size(width, height))?;
    insert_magic_chunk(&png, &payload)
}

pub fn decode_png(bytes: &[u8]) -> Result<Capsule, String> {
    decode_png_with_key(bytes, None)
}

pub fn decode_png_with_key(bytes: &[u8], key: Option<&MagicKey>) -> Result<Capsule, String> {
    if let Some(payload) = magic_chunk(bytes)? {
        return unpack(&payload, key);
    }
    // Backward compatibility: old exported PNGs encoded the source in RGB LSBs.
    archive::decode_png_with_key(bytes, key)
}

pub fn decode_image_with_key(bytes: &[u8], key: Option<&MagicKey>) -> Result<Capsule, String> {
    if bytes.starts_with(b"\x89PNG") {
        decode_png_with_key(bytes, key)
    } else {
        // Existing SVG/SVGZ artifacts remain recoverable, but new transmutations
        // are emitted only as the canonical perceptual PNG.
        archive::decode_image_with_key(bytes, key)
    }
}
