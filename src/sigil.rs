//! Visible, lossless UTF-8 names encoded as normalized radial contours.
//! No hash lookup or name metadata is needed to recover a name.
use crate::MagicKey;
use serde::{Deserialize, Serialize};
use std::f64::consts::PI;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct NameSigil {
    pub points: Vec<[f64; 2]>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct DecodedName {
    pub entity_id: usize,
    pub name: String,
}
fn checksum(bytes: &[u8]) -> u32 {
    let mut crc = !0_u32;
    for &byte in bytes {
        crc ^= byte as u32;
        for _ in 0..8 {
            crc = (crc >> 1) ^ (0xedb88320_u32 & (0_u32.wrapping_sub(crc & 1)));
        }
    }
    !crc
}
pub fn encode_name(name: &str) -> NameSigil {
    encode_name_with_key(name, None).expect("unkeyed contour encoding")
}
pub fn encode_name_with_key(name: &str, key: Option<&MagicKey>) -> Result<NameSigil, String> {
    let encrypted;
    let data = if let Some(key) = key {
        encrypted = key.seal(b"name-contour/v2", name.as_bytes())?;
        encrypted.as_slice()
    } else {
        name.as_bytes()
    };
    let mut frame = Vec::with_capacity(data.len() + 11);
    frame.extend_from_slice(&[0xc7, 0x51, if key.is_some() { 2 } else { 1 }]);
    frame.extend_from_slice(&(data.len() as u32).to_be_bytes());
    frame.extend_from_slice(data);
    frame.extend_from_slice(&checksum(data).to_be_bytes());
    let length = frame.len() * 2;
    let points = frame
        .into_iter()
        .flat_map(|b| [b >> 4, b & 15])
        .enumerate()
        .map(|(i, nibble)| {
            let angle = -PI / 2. + i as f64 * 2. * PI / length as f64;
            let radius = 0.72 + nibble as f64 * 0.014;
            [
                (radius * angle.cos() * 1_000_000.).round() / 1_000_000.,
                (radius * angle.sin() * 1_000_000.).round() / 1_000_000.,
            ]
        })
        .collect();
    Ok(NameSigil { points })
}
pub fn decode_name(sigil: &NameSigil) -> Result<String, String> {
    decode_name_with_key(sigil, None)
}
pub fn decode_name_with_key(sigil: &NameSigil, key: Option<&MagicKey>) -> Result<String, String> {
    let count = sigil.points.len();
    if count < 22 || !count.is_multiple_of(2) {
        return Err("Invalid sigil point count".into());
    }
    let mut nibbles = Vec::with_capacity(count);
    for (i, &[x, y]) in sigil.points.iter().enumerate() {
        if !x.is_finite() || !y.is_finite() {
            return Err("Non-finite sigil coordinate".into());
        }
        let radial = x.hypot(y);
        let nibble = ((radial - 0.72) / 0.014).round();
        if !(0. ..=15.).contains(&nibble) {
            return Err("Invalid radial symbol".into());
        }
        let radius = 0.72 + nibble * 0.014;
        let angle = -PI / 2. + i as f64 * 2. * PI / count as f64;
        if (x - radius * angle.cos()).abs() > 0.000002
            || (y - radius * angle.sin()).abs() > 0.000002
        {
            return Err("Contour geometry has been altered".into());
        }
        nibbles.push(nibble as u8);
    }
    let frame: Vec<u8> = nibbles
        .as_chunks::<2>()
        .0
        .iter()
        .map(|p| (p[0] << 4) | p[1])
        .collect();
    if frame[..2] != [0xc7, 0x51] || !matches!(frame[2], 1 | 2) {
        return Err("Unrecognized sigil format".into());
    }
    let length = u32::from_be_bytes(frame[3..7].try_into().unwrap()) as usize;
    if length != frame.len() - 11 {
        return Err("Sigil length mismatch".into());
    }
    let bytes = &frame[7..7 + length];
    let expected = u32::from_be_bytes(frame[7 + length..].try_into().unwrap());
    if checksum(bytes) != expected {
        return Err("Sigil checksum mismatch".into());
    }
    let plain = if frame[2] == 2 {
        key.ok_or("This name contour requires its original magic key")?
            .open(b"name-contour/v2", bytes)?
    } else {
        bytes.to_vec()
    };
    String::from_utf8(plain).map_err(|_| "Sigil name is not valid UTF-8".into())
}
impl NameSigil {
    pub fn svg_points(&self) -> String {
        self.points
            .iter()
            .map(|p| format!("{:.6},{:.6}", p[0], p[1]))
            .collect::<Vec<_>>()
            .join(" ")
    }
}
fn attribute<'a>(tag: &'a str, name: &str) -> Option<&'a str> {
    let pattern = format!(" {name}=");
    let rest = tag.get(tag.find(&pattern)? + pattern.len()..)?;
    let quote = rest.chars().next()?;
    if quote != '\'' && quote != '"' {
        return None;
    }
    let rest = &rest[1..];
    Some(&rest[..rest.find(quote)?])
}
/// Decode the normalized polygon coordinates in an unmodified Code Is Magic SVG.
/// Plaintext labels, analysis JSON, and font data are not consulted.
pub fn decode_svg_names(svg: &str) -> Result<Vec<DecodedName>, String> {
    decode_svg_names_with_key(svg, None)
}
pub fn decode_svg_names_with_key(
    svg: &str,
    key: Option<&MagicKey>,
) -> Result<Vec<DecodedName>, String> {
    let mut decoded = Vec::new();
    for segment in svg.split("<polygon").skip(1) {
        let tag = segment.split('>').next().ok_or("Unclosed polygon")?;
        if attribute(tag, "data-name-sigil") != Some("1") {
            continue;
        }
        let entity_id = attribute(tag, "data-entity")
            .ok_or("Missing entity identifier")?
            .parse()
            .map_err(|_| "Invalid entity identifier")?;
        let mut points = Vec::new();
        for pair in attribute(tag, "points")
            .ok_or("Missing contour geometry")?
            .split_whitespace()
        {
            let (x, y) = pair.split_once(',').ok_or("Invalid point")?;
            points.push([
                x.parse().map_err(|_| "Invalid x coordinate")?,
                y.parse().map_err(|_| "Invalid y coordinate")?,
            ]);
        }
        let name = decode_name_with_key(&NameSigil { points }, key)?;
        decoded.push(DecodedName { entity_id, name });
    }
    if decoded.is_empty() {
        return Err("No reversible name contours found".into());
    }
    Ok(decoded)
}
