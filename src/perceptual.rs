//! Canonical visually-imperceptible magic export.
//!
//! Code Is Magic artwork is synthetic vector geometry, so JPEG-style raster loss is
//! wasteful: the exact SVGZ was already smaller than high-quality JPEG in the project
//! reference drawing. The canonical export therefore applies deterministic sub-pixel
//! geometry quantization, keeps source/name recovery exact, then writes reproducible
//! SVGZ. Path geometry is rounded to 0.01 SVG units and placement geometry to 0.001
//! units. Reversible name-sigil polygon and placement geometry is left byte-for-byte
//! unchanged.
use crate::{compact_svg, embed_svg_with_key, Capsule, MagicKey};
use std::{io::Write, ops::Range};

const PATH_DECIMALS: usize = 2;
const PLACEMENT_DECIMALS: usize = 3;

fn trim_number(mut text: String) -> String {
    if let Some(dot) = text.find('.') {
        while text.ends_with('0') {
            text.pop();
        }
        if text.len() == dot + 1 {
            text.pop();
        }
    }
    if text == "-0" || text == "+0" || text.is_empty() {
        "0".into()
    } else {
        text
    }
}

fn quantize_numbers(input: &str, decimals: usize) -> String {
    let bytes = input.as_bytes();
    let mut out = String::with_capacity(input.len());
    let mut i = 0;
    while i < bytes.len() {
        let start = i;
        let mut j = i;
        if matches!(bytes[j], b'+' | b'-') {
            j += 1;
        }
        let int_start = j;
        while j < bytes.len() && bytes[j].is_ascii_digit() {
            j += 1;
        }
        let mut has_digits = j > int_start;
        let mut has_fraction = false;
        if j < bytes.len() && bytes[j] == b'.' {
            has_fraction = true;
            j += 1;
            let fraction_start = j;
            while j < bytes.len() && bytes[j].is_ascii_digit() {
                j += 1;
            }
            has_digits |= j > fraction_start;
        }
        let exponent_start = j;
        if has_digits && j < bytes.len() && matches!(bytes[j], b'e' | b'E') {
            j += 1;
            if j < bytes.len() && matches!(bytes[j], b'+' | b'-') {
                j += 1;
            }
            let digits = j;
            while j < bytes.len() && bytes[j].is_ascii_digit() {
                j += 1;
            }
            if j == digits {
                j = exponent_start;
            }
        }
        let begins_number = has_digits
            && (start == 0
                || !matches!(bytes[start - 1], b'0'..=b'9' | b'.' | b'e' | b'E'));
        if begins_number && j > start {
            let token = &input[start..j];
            if has_fraction || token.contains('e') || token.contains('E') {
                if let Ok(value) = token.parse::<f64>() {
                    out.push_str(&trim_number(format!("{value:.decimals$}")));
                    i = j;
                    continue;
                }
            }
        }
        let ch = input[i..].chars().next().unwrap();
        out.push(ch);
        i += ch.len_utf8();
    }
    out
}

fn replacement(
    svg: &str,
    attribute: roxmltree::Attribute<'_, '_>,
    decimals: usize,
) -> Option<(Range<usize>, String)> {
    let value = quantize_numbers(attribute.value(), decimals);
    (value != attribute.value()).then(|| {
        let range = attribute.range();
        let original = &svg[range.clone()];
        let quote = if original.contains('"') { '"' } else { '\'' };
        (
            range,
            format!("{}={quote}{value}{quote}", attribute.name()),
        )
    })
}

/// Lossily reduce numeric precision below a visible pixel while preserving semantic
/// recovery geometry and all non-geometry data. The transform is deterministic and
/// intentionally conservative: a path coordinate moves by at most 0.005 SVG units.
pub fn perceptual_svg(svg: &str) -> Result<String, String> {
    let compact = compact_svg(svg)?;
    let document = roxmltree::Document::parse(&compact).map_err(|_| "Invalid SVG artwork")?;
    if !document.root_element().has_tag_name("svg") {
        return Err("Invalid SVG artwork".into());
    }
    let placement = [
        "transform",
        "x",
        "y",
        "cx",
        "cy",
        "r",
        "rx",
        "ry",
        "x1",
        "y1",
        "x2",
        "y2",
        "width",
        "height",
        "stroke-width",
    ];
    let mut replacements = Vec::new();
    for node in document.descendants().filter(|node| node.is_element()) {
        let name_sigil = node.attribute("data-name-sigil") == Some("1");
        if node.has_tag_name("path") && !name_sigil {
            if let Some(attribute) = node.attribute_node("d") {
                if let Some(edit) = replacement(&compact, attribute, PATH_DECIMALS) {
                    replacements.push(edit);
                }
            }
        }
        // The name contour is an independent reversible channel. Preserve every
        // geometric attribute on that element exactly, not merely its polygon points.
        if !name_sigil {
            for name in placement {
                if let Some(attribute) = node.attribute_node(name) {
                    if let Some(edit) = replacement(&compact, attribute, PLACEMENT_DECIMALS) {
                        replacements.push(edit);
                    }
                }
            }
        }
    }
    replacements.sort_by_key(|(range, _)| range.start);
    for pair in replacements.windows(2) {
        if pair[0].0.end > pair[1].0.start {
            return Err("Overlapping SVG geometry attributes".into());
        }
    }
    let mut result = compact;
    for (range, value) in replacements.into_iter().rev() {
        result.replace_range(range, &value);
    }
    Ok(result)
}

/// The only canonical transmutation format: visually-imperceptible SVGZ plus an
/// exact, optionally authenticated/encrypted source capsule.
pub fn encode_magic_with_key(
    capsule: &Capsule,
    key: Option<&MagicKey>,
) -> Result<Vec<u8>, String> {
    let mut perceptual = capsule.clone();
    perceptual.svg = perceptual_svg(&perceptual.svg)?;
    let svg = embed_svg_with_key(&perceptual, key)?;
    let mut encoder = flate2::GzBuilder::new()
        .mtime(0)
        .write(Vec::new(), flate2::Compression::best());
    encoder
        .write_all(svg.as_bytes())
        .map_err(|e| e.to_string())?;
    encoder.finish().map_err(|e| e.to_string())
}

pub fn encode_magic(capsule: &Capsule) -> Result<Vec<u8>, String> {
    encode_magic_with_key(capsule, None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{decode_image_with_key, ArchiveEntry, Snapshot};

    fn capsule() -> Capsule {
        Capsule {
            version: 1,
            snapshot: Snapshot {
                version: 1,
                title: "fixture".into(),
                commit: None,
                entries: vec![ArchiveEntry {
                    path: "main.rs".into(),
                    mode: "100644".into(),
                    data: "Zm4gbWFpbigpIHt9Cg==".into(),
                    sha: None,
                }],
            },
            svg: "<svg xmlns='http://www.w3.org/2000/svg' width='100.123456' height='100.987654'><path d='M0.123456 1.987654L99.876543 98.012345'/><polygon data-name-sigil='1' data-entity='0' data-seal-radius='8.123456' points='0.123456,0.987654 1.111111,2.222222' transform='translate(50.123456 49.987654) scale(8.123456)'/></svg>".into(),
        }
    }

    #[test]
    fn rounds_only_non_recovery_subpixel_geometry() {
        let svg = perceptual_svg(&capsule().svg).unwrap();
        assert!(svg.contains("width='100.123'"));
        assert!(svg.contains("height='100.988'"));
        assert!(svg.contains("d='M0.12 1.99L99.88 98.01'"));
        assert!(svg.contains("transform='translate(50.123456 49.987654) scale(8.123456)'"));
        assert!(svg.contains("points='0.123456,0.987654 1.111111,2.222222'"));
        assert!(svg.contains("data-seal-radius='8.123456'"));
    }

    #[test]
    fn canonical_export_is_deterministic_and_source_exact() {
        let capsule = capsule();
        let first = encode_magic(&capsule).unwrap();
        assert_eq!(first, encode_magic(&capsule).unwrap());
        let recovered = decode_image_with_key(&first, None).unwrap();
        assert_eq!(recovered.snapshot, capsule.snapshot);
        assert_eq!(recovered.svg, perceptual_svg(&capsule.svg).unwrap());
    }
}
