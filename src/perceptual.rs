//! Deterministic visually-lossless JPEG carrier.
//!
//! The displayed raster is deliberately perceptual rather than pixel-exact. The repository
//! snapshot is compressed and stored independently in APP15 segments, so source recovery
//! remains byte-exact and does not depend on lossy pixels surviving a round trip.
use crate::{archive::crc32, sigil::decode_svg_names_with_key, Capsule, MagicKey, Snapshot};
use jpeg_encoder::{ColorType, Encoder, SamplingFactor};
use serde::{Deserialize, Serialize};
use std::io::{Read, Write};

const MAX_ARCHIVE: usize = 128_000_000;
const MAX_PIXELS: usize = 16_000_000;
const APP_MAGIC: &[u8; 8] = b"CIMJPG01";
const BROTLI_MAGIC: &[u8; 8] = b"CIMVBR01";
const KEY_MAGIC: &[u8; 8] = b"CIMVKEY1";
const KEY_DOMAIN: &[u8] = b"perceptual-source/v1";
const APP_HEADER: usize = 20;
const APP_DATA: usize = 65_000;
const MAX_CHUNKS: usize = 4096;

/// Conservative deterministic profile for line art, runes, thin connections and glow edges.
pub const DEFAULT_PERCEPTUAL_QUALITY: u8 = 96;

#[derive(Serialize, Deserialize)]
struct RasterSource {
    version: u32,
    snapshot: Snapshot,
}

fn pack_source(snapshot: &Snapshot) -> Result<Vec<u8>, String> {
    let mut snapshot = snapshot.clone();
    snapshot.canonicalize()?;
    let raw = serde_json::to_vec(&RasterSource {
        version: 1,
        snapshot,
    })
    .map_err(|e| e.to_string())?;
    if raw.len() > MAX_ARCHIVE {
        return Err("Archive too large".into());
    }

    let mut z = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::best());
    z.write_all(&raw).map_err(|e| e.to_string())?;
    let deflate = z.finish().map_err(|e| e.to_string())?;

    let mut brotli = BROTLI_MAGIC.to_vec();
    let mut reader = brotli::CompressorReader::new(raw.as_slice(), 4096, 9, 22);
    reader.read_to_end(&mut brotli).map_err(|e| e.to_string())?;
    Ok(if brotli.len() < deflate.len() {
        brotli
    } else {
        deflate
    })
}

fn unpack_source(bytes: &[u8]) -> Result<Snapshot, String> {
    let mut raw = Vec::new();
    let reader: Box<dyn Read + '_> = if let Some(data) = bytes.strip_prefix(BROTLI_MAGIC) {
        Box::new(brotli::Decompressor::new(data, 4096))
    } else {
        Box::new(flate2::read::ZlibDecoder::new(bytes))
    };
    reader
        .take((MAX_ARCHIVE + 1) as u64)
        .read_to_end(&mut raw)
        .map_err(|_| "Corrupt compressed perceptual source capsule")?;
    if raw.len() > MAX_ARCHIVE {
        return Err("Decoded archive exceeds budget".into());
    }
    let mut source: RasterSource =
        serde_json::from_slice(&raw).map_err(|_| "Invalid perceptual source capsule")?;
    if source.version != 1 {
        return Err("Unsupported perceptual source capsule version".into());
    }
    source.snapshot.canonicalize()?;
    Ok(source.snapshot)
}

fn protect_source(snapshot: &Snapshot, key: Option<&MagicKey>) -> Result<Vec<u8>, String> {
    let packed = pack_source(snapshot)?;
    if let Some(key) = key {
        let mut data = KEY_MAGIC.to_vec();
        data.extend(key.seal(KEY_DOMAIN, &packed)?);
        Ok(data)
    } else {
        Ok(packed)
    }
}

fn unprotect_source(bytes: &[u8], key: Option<&MagicKey>) -> Result<Snapshot, String> {
    if let Some(encrypted) = bytes.strip_prefix(KEY_MAGIC) {
        let key = key.ok_or("This image requires its original magic key")?;
        unpack_source(&key.open(KEY_DOMAIN, encrypted)?)
    } else {
        unpack_source(bytes)
    }
}

fn app_segments(payload: &[u8]) -> Result<Vec<Vec<u8>>, String> {
    if payload.len() > MAX_ARCHIVE {
        return Err("Perceptual source capsule exceeds budget".into());
    }
    let total = payload.len().div_ceil(APP_DATA).max(1);
    if total > MAX_CHUNKS || total > u16::MAX as usize {
        return Err("Perceptual source capsule needs too many JPEG segments".into());
    }
    let total_len = u32::try_from(payload.len()).map_err(|_| "Source capsule is too large")?;
    let crc = crc32(payload);
    let mut segments = Vec::with_capacity(total);
    for sequence in 0..total {
        let start = sequence * APP_DATA;
        let end = (start + APP_DATA).min(payload.len());
        let mut segment = Vec::with_capacity(APP_HEADER + end - start);
        segment.extend_from_slice(APP_MAGIC);
        segment.extend_from_slice(&(sequence as u16).to_be_bytes());
        segment.extend_from_slice(&(total as u16).to_be_bytes());
        segment.extend_from_slice(&total_len.to_be_bytes());
        segment.extend_from_slice(&crc.to_be_bytes());
        segment.extend_from_slice(&payload[start..end]);
        segments.push(segment);
    }
    Ok(segments)
}

fn extract_payload(bytes: &[u8]) -> Result<Vec<u8>, String> {
    if !bytes.starts_with(&[0xff, 0xd8]) {
        return Err("Not a JPEG image".into());
    }
    let mut pos = 2;
    let mut pieces: Option<Vec<Option<Vec<u8>>>> = None;
    let mut expected_len = 0_usize;
    let mut expected_crc = 0_u32;

    while pos + 1 < bytes.len() {
        if bytes[pos] != 0xff {
            return Err("Invalid JPEG marker stream".into());
        }
        while pos < bytes.len() && bytes[pos] == 0xff {
            pos += 1;
        }
        if pos >= bytes.len() {
            break;
        }
        let marker = bytes[pos];
        pos += 1;
        if marker == 0xda || marker == 0xd9 {
            break;
        }
        if marker == 0x01 || (0xd0..=0xd7).contains(&marker) {
            continue;
        }
        if pos + 2 > bytes.len() {
            return Err("Truncated JPEG marker".into());
        }
        let length = u16::from_be_bytes([bytes[pos], bytes[pos + 1]]) as usize;
        if length < 2 || pos + length > bytes.len() {
            return Err("Invalid JPEG segment length".into());
        }
        let data = &bytes[pos + 2..pos + length];
        pos += length;

        if marker != 0xef || !data.starts_with(APP_MAGIC) {
            continue;
        }
        if data.len() < APP_HEADER {
            return Err("Truncated Code Is Magic JPEG segment".into());
        }
        let sequence = u16::from_be_bytes(data[8..10].try_into().unwrap()) as usize;
        let total = u16::from_be_bytes(data[10..12].try_into().unwrap()) as usize;
        let total_len = u32::from_be_bytes(data[12..16].try_into().unwrap()) as usize;
        let crc = u32::from_be_bytes(data[16..20].try_into().unwrap());
        if total == 0 || total > MAX_CHUNKS || sequence >= total || total_len > MAX_ARCHIVE {
            return Err("Invalid Code Is Magic JPEG segment index".into());
        }
        match &mut pieces {
            None => {
                pieces = Some(vec![None; total]);
                expected_len = total_len;
                expected_crc = crc;
            }
            Some(parts)
                if parts.len() != total || expected_len != total_len || expected_crc != crc =>
            {
                return Err("Inconsistent Code Is Magic JPEG segments".into());
            }
            _ => {}
        }
        let parts = pieces.as_mut().unwrap();
        if parts[sequence].is_some() {
            return Err("Duplicate Code Is Magic JPEG segment".into());
        }
        parts[sequence] = Some(data[APP_HEADER..].to_vec());
    }

    let parts = pieces.ok_or("No recoverable Code Is Magic source capsule found in JPEG")?;
    if parts.iter().any(Option::is_none) {
        return Err("Source capsule damaged; a JPEG segment is missing".into());
    }
    let mut payload = Vec::with_capacity(expected_len);
    for part in parts {
        payload.extend_from_slice(&part.unwrap());
    }
    if payload.len() != expected_len || crc32(&payload) != expected_crc {
        return Err("Source capsule damaged; exact recovery is impossible".into());
    }
    Ok(payload)
}

fn render_rgb(svg: &str) -> Result<(Vec<u8>, u16, u16), String> {
    let tree = resvg::usvg::Tree::from_str(svg, &resvg::usvg::Options::default())
        .map_err(|_| "Invalid vector artwork")?;
    let size = tree.size();
    let natural = (size.width() as usize)
        .checked_mul(size.height() as usize)
        .ok_or("Artwork dimensions overflow")?;
    let pixels = natural.min(MAX_PIXELS).max(64 * 64);
    let aspect = (size.width() as f64 / size.height() as f64).clamp(0.1, 10.0);
    let width = ((pixels as f64 * aspect).sqrt().ceil() as u32).max(64);
    let height = (pixels.div_ceil(width as usize) as u32).max(64);
    if width as usize * height as usize > MAX_PIXELS
        || width > u16::MAX as u32
        || height > u16::MAX as u32
    {
        return Err("Perceptual image dimensions exceed budget".into());
    }
    let mut pixmap =
        resvg::tiny_skia::Pixmap::new(width, height).ok_or("Could not allocate JPEG raster")?;
    resvg::render(
        &tree,
        resvg::tiny_skia::Transform::from_scale(
            width as f32 / size.width(),
            height as f32 / size.height(),
        ),
        &mut pixmap.as_mut(),
    );
    let rgba = pixmap.data();
    if rgba.as_chunks::<4>().0.iter().any(|p| p[3] != 255) {
        return Err("Visually-lossless JPEG export requires opaque artwork".into());
    }
    let mut rgb = Vec::with_capacity(width as usize * height as usize * 3);
    for pixel in rgba.as_chunks::<4>().0 {
        rgb.extend_from_slice(&pixel[..3]);
    }
    Ok((rgb, width as u16, height as u16))
}

/// Encode a deterministic, visually-lossless JPEG while preserving the repository exactly.
/// Quality 96 with 4:4:4 chroma is the default profile; 95..=100 is accepted explicitly.
pub fn encode_perceptual_jpeg_with_key(
    capsule: &Capsule,
    key: Option<&MagicKey>,
    quality: u8,
) -> Result<Vec<u8>, String> {
    if !(95..=100).contains(&quality) {
        return Err("Perceptual JPEG quality must be between 95 and 100".into());
    }
    // Name contours are the key-sensitive part of keyed artwork. Decode them before export so
    // a caller can never accidentally turn a keyed diagram into a plaintext-source JPEG by
    // omitting or supplying the wrong key. Unkeyed contours decode successfully with any key.
    if capsule.svg.contains("data-name-sigil=") {
        decode_svg_names_with_key(&capsule.svg, key)?;
    }
    let payload = protect_source(&capsule.snapshot, key)?;
    let (rgb, width, height) = render_rgb(&capsule.svg)?;
    let mut jpeg = Vec::new();
    let mut encoder = Encoder::new(&mut jpeg, quality);
    encoder.set_sampling_factor(SamplingFactor::F_1_1);
    encoder.set_optimized_huffman_tables(true);
    encoder.set_progressive(true);
    for segment in app_segments(&payload)? {
        encoder
            .add_app_segment(15, segment)
            .map_err(|e| e.to_string())?;
    }
    encoder
        .encode(&rgb, width, height, ColorType::Rgb)
        .map_err(|e| e.to_string())?;
    Ok(jpeg)
}

pub fn encode_perceptual_jpeg(capsule: &Capsule) -> Result<Vec<u8>, String> {
    encode_perceptual_jpeg_with_key(capsule, None, DEFAULT_PERCEPTUAL_QUALITY)
}

/// Recover the exact source snapshot from an original Code Is Magic perceptual JPEG.
/// The returned capsule intentionally has no exact SVG because the displayed raster is lossy;
/// callers can deterministically re-render the snapshot when vector artwork is desired.
pub fn decode_perceptual_jpeg_with_key(
    bytes: &[u8],
    key: Option<&MagicKey>,
) -> Result<Capsule, String> {
    let snapshot = unprotect_source(&extract_payload(bytes)?, key)?;
    Ok(Capsule {
        version: 1,
        snapshot,
        svg: String::new(),
    })
}

pub fn decode_perceptual_jpeg(bytes: &[u8]) -> Result<Capsule, String> {
    decode_perceptual_jpeg_with_key(bytes, None)
}
