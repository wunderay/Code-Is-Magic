use crate::{supported, MagicKey, SourceFile};
use base64::Engine;
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeSet,
    io::{Cursor, Read, Write},
};
const MAX_ARCHIVE: usize = 128_000_000;
const MAX_PIXELS: usize = 64_000_000;
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ArchiveEntry {
    pub path: String,
    pub mode: String,
    pub data: String,
    #[serde(default)]
    pub sha: Option<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Snapshot {
    pub version: u32,
    pub title: String,
    pub commit: Option<String>,
    pub entries: Vec<ArchiveEntry>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Capsule {
    pub version: u32,
    pub snapshot: Snapshot,
    pub svg: String,
}
pub fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = !0_u32;
    for &b in bytes {
        crc ^= b as u32;
        for _ in 0..8 {
            crc = (crc >> 1) ^ (0xedb88320_u32 & (0_u32.wrapping_sub(crc & 1)));
        }
    }
    !crc
}
impl Snapshot {
    pub fn canonicalize(&mut self) -> Result<(), String> {
        if self.version != 1 {
            return Err("Unsupported snapshot version".into());
        }
        if self.entries.len() > 5000 {
            return Err("Snapshot exceeds 5,000 entries".into());
        }
        self.entries.sort_by(|a, b| a.path.cmp(&b.path));
        let mut names = BTreeSet::new();
        let mut total = 0;
        for e in &self.entries {
            if e.path.is_empty()
                || e.path.starts_with('/')
                || e.path.contains('\\')
                || e.path.contains('\0')
                || e.path
                    .split('/')
                    .any(|p| p.is_empty() || p == "." || p == ".." || p.contains(':'))
            {
                return Err("Unsafe archive path".into());
            }
            if !names.insert(&e.path) {
                return Err("Duplicate archive path".into());
            }
            if !matches!(e.mode.as_str(), "100644" | "100755" | "120000" | "160000") {
                return Err("Invalid repository file mode".into());
            }
            let bytes = base64::engine::general_purpose::STANDARD
                .decode(&e.data)
                .map_err(|_| "Invalid file encoding")?;
            if bytes.len() > 2_000_000 {
                return Err("File exceeds 2 MB".into());
            }
            total += bytes.len();
            if total > 50_000_000 {
                return Err("Snapshot exceeds 50 MB".into());
            }
            if e.mode == "160000"
                && (e
                    .sha
                    .as_ref()
                    .is_none_or(|s| s.len() != 40 || !s.bytes().all(|b| b.is_ascii_hexdigit()))
                    || !bytes.is_empty())
            {
                return Err("Invalid submodule pointer".into());
            }
        }
        Ok(())
    }
    pub fn source_files(&self) -> Vec<SourceFile> {
        self.entries
            .iter()
            .filter(|e| e.mode.starts_with("100") && supported(&e.path))
            .filter_map(|e| {
                let bytes = base64::engine::general_purpose::STANDARD
                    .decode(&e.data)
                    .ok()?;
                Some(SourceFile {
                    path: e.path.clone(),
                    content: String::from_utf8(bytes).ok()?,
                })
            })
            .collect()
    }
}
fn packed(capsule: &Capsule) -> Result<Vec<u8>, String> {
    let mut capsule = capsule.clone();
    capsule.snapshot.canonicalize()?;
    if capsule.version != 1 {
        return Err("Unsupported capsule version".into());
    }
    let raw = serde_json::to_vec(&capsule).map_err(|e| e.to_string())?;
    if raw.len() > MAX_ARCHIVE {
        return Err("Archive too large".into());
    }
    let mut z = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::new(6));
    z.write_all(&raw).map_err(|e| e.to_string())?;
    z.finish().map_err(|e| e.to_string())
}
fn unpack(bytes: &[u8]) -> Result<Capsule, String> {
    let mut raw = Vec::new();
    flate2::read::ZlibDecoder::new(bytes)
        .take((MAX_ARCHIVE + 1) as u64)
        .read_to_end(&mut raw)
        .map_err(|_| "Corrupt compressed archive")?;
    if raw.len() > MAX_ARCHIVE {
        return Err("Decoded archive exceeds budget".into());
    }
    let mut capsule: Capsule = serde_json::from_slice(&raw).map_err(|_| "Invalid archive")?;
    if capsule.version != 1 {
        return Err("Unsupported capsule version".into());
    }
    capsule.snapshot.canonicalize()?;
    Ok(capsule)
}
const ARCHIVE_DOMAIN: &[u8] = b"source-capsule/v1";
fn protect(capsule: &Capsule, key: Option<&MagicKey>) -> Result<Vec<u8>, String> {
    // A keyed contour must never be exported with an unencrypted source capsule.
    if capsule.svg.contains("data-magic-key='1'") && key.is_none() {
        return Err("A magic key is required to export this keyed artwork".into());
    }
    if capsule.svg.contains("data-magic-key='1'") && capsule.svg.contains("data-name-sigil='1'") {
        // Do not accidentally encrypt the source with a different key than its contours.
        crate::decode_svg_names_with_key(&capsule.svg, key)?;
    }
    let packed = packed(capsule)?;
    if let Some(key) = key {
        let mut data = b"CIMKEY01".to_vec();
        data.extend(key.seal(ARCHIVE_DOMAIN, &packed)?);
        if data.len() > MAX_ARCHIVE {
            return Err("Encrypted archive exceeds budget".into());
        }
        Ok(data)
    } else {
        Ok(packed)
    }
}
fn unprotect(bytes: &[u8], key: Option<&MagicKey>) -> Result<Capsule, String> {
    if let Some(encrypted) = bytes.strip_prefix(b"CIMKEY01") {
        let key = key.ok_or("This image requires its original magic key")?;
        unpack(&key.open(ARCHIVE_DOMAIN, encrypted)?)
    } else {
        unpack(bytes)
    }
}
/// Exact source bytes live in the low three bits of RGB pixels, not PNG metadata.
pub fn encode_png(capsule: &Capsule) -> Result<Vec<u8>, String> {
    encode_png_with_key(capsule, None)
}
pub fn encode_png_with_key(capsule: &Capsule, key: Option<&MagicKey>) -> Result<Vec<u8>, String> {
    let data = protect(capsule, key)?;
    let mut frame = b"CIMPNG01".to_vec();
    frame.extend_from_slice(&(data.len() as u64).to_be_bytes());
    frame.extend_from_slice(&crc32(&data).to_be_bytes());
    frame.extend_from_slice(&data);
    let options = resvg::usvg::Options::default();
    let tree = resvg::usvg::Tree::from_str(&capsule.svg, &options)
        .map_err(|_| "Invalid vector artwork")?;
    let size = tree.size();
    let needed = (frame.len() * 8).div_ceil(9);
    let aspect = (size.width() as f64 / size.height() as f64).clamp(0.1, 10.);
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
    let rgba = pixmap.data_mut();
    let bits = frame.len() * 8;
    let mut bit = 0;
    for pixel in rgba.as_chunks_mut::<4>().0 {
        // All generated artwork has an opaque background.
        pixel[3] = 255;
        for channel in &mut pixel[..3] {
            if bit >= bits {
                break;
            }
            let mut value = 0;
            for _ in 0..3 {
                value <<= 1;
                if bit < bits {
                    value |= (frame[bit / 8] >> (7 - bit % 8)) & 1;
                }
                bit += 1;
            }
            *channel = (*channel & 0xf8) | value;
        }
    }
    if bit < bits {
        return Err("Image has insufficient pixel capacity".into());
    }
    let mut bytes = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut bytes, width, height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        encoder.set_compression(png::Compression::Best);
        encoder
            .write_header()
            .map_err(|e| e.to_string())?
            .write_image_data(rgba)
            .map_err(|e| e.to_string())?;
    }
    Ok(bytes)
}
pub fn decode_png(bytes: &[u8]) -> Result<Capsule, String> {
    decode_png_with_key(bytes, None)
}
pub fn decode_png_with_key(bytes: &[u8], key: Option<&MagicKey>) -> Result<Capsule, String> {
    let mut decoder = png::Decoder::new(Cursor::new(bytes));
    decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
    decoder.set_limits(png::Limits {
        bytes: MAX_PIXELS * 4,
    });
    let mut reader = decoder.read_info().map_err(|_| "Invalid PNG image")?;
    let info = reader.info();
    if (info.width as usize)
        .checked_mul(info.height as usize)
        .ok_or("Image dimensions overflow")?
        > MAX_PIXELS
    {
        return Err("Image exceeds pixel budget".into());
    }
    let mut rgba = vec![0; reader.output_buffer_size()];
    let info = reader
        .next_frame(&mut rgba)
        .map_err(|_| "PNG data is corrupt")?;
    let channels = match info.color_type {
        png::ColorType::Rgb => 3,
        png::ColorType::Rgba => 4,
        _ => return Err("This image has no recoverable RGB source capsule".into()),
    };
    let capacity = info.width as usize * info.height as usize * 9 / 8;
    let mut frame = Vec::new();
    let (mut byte, mut bits, mut expected) = (0_u8, 0_usize, 20_usize);
    'pixels: for pixel in rgba[..info.buffer_size()].chunks_exact(channels) {
        for &channel in &pixel[..3] {
            for shift in (0..3).rev() {
                byte = (byte << 1) | ((channel >> shift) & 1);
                bits += 1;
                if bits == 8 {
                    frame.push(byte);
                    byte = 0;
                    bits = 0;
                    if frame.len() == 20 {
                        if &frame[..8] != b"CIMPNG01" {
                            return Err("No source capsule found. Use the original exported PNG, without resizing or JPEG conversion.".into());
                        }
                        let length = u64::from_be_bytes(frame[8..16].try_into().unwrap());
                        if length > MAX_ARCHIVE as u64 || length as usize + 20 > capacity {
                            return Err("Invalid capsule length".into());
                        }
                        expected = length as usize + 20;
                    }
                    if frame.len() == expected {
                        break 'pixels;
                    }
                }
            }
        }
    }
    if frame.len() != expected || frame.len() < 20 {
        return Err("Truncated source capsule".into());
    }
    if crc32(&frame[20..]) != u32::from_be_bytes(frame[16..20].try_into().unwrap()) {
        return Err("Source capsule damaged; exact recovery is impossible".into());
    }
    unprotect(&frame[20..], key)
}
pub fn embed_svg(capsule: &Capsule) -> Result<String, String> {
    embed_svg_with_key(capsule, None)
}
pub fn embed_svg_with_key(capsule: &Capsule, key: Option<&MagicKey>) -> Result<String, String> {
    let data = base64::engine::general_purpose::STANDARD.encode(protect(capsule, key)?);
    let metadata = format!("<metadata id='source-capsule-v1'>{data}</metadata>");
    let index = capsule.svg.rfind("</svg>").ok_or("Invalid SVG")?;
    Ok(format!(
        "{}{}{}",
        &capsule.svg[..index],
        metadata,
        &capsule.svg[index..]
    ))
}
pub fn decode_svg(svg: &str) -> Result<Capsule, String> {
    decode_svg_with_key(svg, None)
}
pub fn decode_svg_with_key(svg: &str, key: Option<&MagicKey>) -> Result<Capsule, String> {
    let open = "<metadata id='source-capsule-v1'>";
    let text = svg
        .split_once(open)
        .ok_or("No source capsule found")?
        .1
        .split_once("</metadata>")
        .ok_or("Truncated SVG capsule")?
        .0;
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(text)
        .map_err(|_| "Invalid SVG capsule")?;
    if bytes.len() > MAX_ARCHIVE {
        return Err("SVG capsule exceeds budget".into());
    }
    let capsule = unprotect(&bytes, key)?;
    if bytes.starts_with(b"CIMKEY01") && embed_svg_with_key(&capsule, key)? != svg {
        return Err("Encrypted SVG artwork has been altered".into());
    }
    Ok(capsule)
}
/// Deterministic ZIP: original files under repository/, provenance beside that folder.
pub fn snapshot_zip(snapshot: &Snapshot) -> Result<Vec<u8>, String> {
    let mut snapshot = snapshot.clone();
    snapshot.canonicalize()?;
    let mut files = Vec::new();
    for e in &snapshot.entries {
        if e.mode != "160000" {
            files.push((
                format!("repository/{}", e.path),
                base64::engine::general_purpose::STANDARD
                    .decode(&e.data)
                    .map_err(|_| "Invalid file bytes")?,
                match e.mode.as_str() {
                    "100755" => 0o100755,
                    "120000" => 0o120777,
                    _ => 0o100644,
                },
            ));
        }
    }
    // The gitlink object IDs are retained here; external submodule contents are not in the parent tree.
    files.push((
        "restoration.json".into(),
        serde_json::to_vec(&snapshot).map_err(|e| e.to_string())?,
        0o100644,
    ));
    let mut out = Vec::new();
    let mut central = Vec::new();
    for (name, data, mode) in &files {
        if name.len() > u16::MAX as usize {
            return Err("ZIP path too long".into());
        }
        let offset = out.len() as u32;
        let crc = crc32(data);
        let length = data.len() as u32;
        put32(&mut out, 0x04034b50);
        for v in [20, 0x800, 0, 0, 33] {
            put16(&mut out, v);
        }
        for v in [crc, length, length] {
            put32(&mut out, v);
        }
        put16(&mut out, name.len() as u16);
        put16(&mut out, 0);
        out.extend_from_slice(name.as_bytes());
        out.extend_from_slice(data);
        put32(&mut central, 0x02014b50);
        for v in [0x0314, 20, 0x800, 0, 0, 33] {
            put16(&mut central, v);
        }
        for v in [crc, length, length] {
            put32(&mut central, v);
        }
        for v in [name.len() as u16, 0, 0, 0, 0] {
            put16(&mut central, v);
        }
        put32(&mut central, *mode << 16);
        put32(&mut central, offset);
        central.extend_from_slice(name.as_bytes());
    }
    let offset = out.len() as u32;
    let size = central.len() as u32;
    out.extend(central);
    put32(&mut out, 0x06054b50);
    for v in [0, 0, files.len() as u16, files.len() as u16] {
        put16(&mut out, v);
    }
    put32(&mut out, size);
    put32(&mut out, offset);
    put16(&mut out, 0);
    Ok(out)
}
fn put16(v: &mut Vec<u8>, n: u16) {
    v.extend_from_slice(&n.to_le_bytes());
}
fn put32(v: &mut Vec<u8>, n: u32) {
    v.extend_from_slice(&n.to_le_bytes());
}
