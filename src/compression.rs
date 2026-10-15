//! Lossless storage selection. No quantization, resampling, or coordinate rounding.
use png::{AdaptiveFilterType, BitDepth, ColorType, FilterType, Info};
use std::{borrow::Cow, collections::BTreeMap, io::Cursor};

fn encode(data: &[u8], info: &Info<'_>, best: &mut Vec<u8>) -> Result<(), String> {
    for filter in [
        None,
        Some(FilterType::NoFilter),
        Some(FilterType::Sub),
        Some(FilterType::Up),
        Some(FilterType::Avg),
        Some(FilterType::Paeth),
    ] {
        let mut bytes = Vec::new();
        let mut encoder =
            png::Encoder::with_info(&mut bytes, info.clone()).map_err(|e| e.to_string())?;
        encoder.set_compression(png::Compression::Best);
        if let Some(filter) = filter {
            encoder.set_filter(filter);
        } else {
            encoder.set_adaptive_filter(AdaptiveFilterType::Adaptive);
        }
        encoder
            .write_header()
            .map_err(|e| e.to_string())?
            .write_image_data(data)
            .map_err(|e| e.to_string())?;
        if best.is_empty() || bytes.len() < best.len() {
            *best = bytes;
        }
    }
    Ok(())
}

pub(crate) fn encode_pixels(rgba: &[u8], mut info: Info<'_>) -> Result<Vec<u8>, String> {
    let pixels = rgba.as_chunks::<4>().0;
    let opaque = pixels.iter().all(|p| p[3] == 255);
    // RGB ICC profiles require RGB or indexed storage, even for gray pixels.
    let gray = info.icc_profile.is_none() && pixels.iter().all(|p| p[0] == p[1] && p[1] == p[2]);
    info.interlaced = false;
    info.bit_depth = BitDepth::Eight;
    info.palette = None;
    info.trns = None;
    info.sbit = None;
    let mut best = Vec::new();
    // Keep the legacy RGBA/Sub encoding as a size floor for every new export.
    info.color_type = ColorType::Rgba;
    {
        let mut encoder =
            png::Encoder::with_info(&mut best, info.clone()).map_err(|e| e.to_string())?;
        encoder.set_compression(png::Compression::Best);
        encoder
            .write_header()
            .map_err(|e| e.to_string())?
            .write_image_data(rgba)
            .map_err(|e| e.to_string())?;
    }
    info.color_type = match (gray, opaque) {
        (true, true) => ColorType::Grayscale,
        (true, false) => ColorType::GrayscaleAlpha,
        (false, true) => ColorType::Rgb,
        (false, false) => ColorType::Rgba,
    };
    let mut data = Vec::with_capacity(pixels.len() * info.color_type.samples());
    for pixel in pixels {
        if gray {
            data.push(pixel[0]);
        } else {
            data.extend_from_slice(&pixel[..3]);
        }
        if !opaque {
            data.push(pixel[3]);
        }
    }
    encode(&data, &info, &mut best)?;

    // An exact palette is eligible only if every distinct RGBA value fits.
    // Encrypted carriers normally have far more colors; stop counting at 257.
    let mut colors = BTreeMap::new();
    for pixel in pixels {
        if !colors.contains_key(pixel) {
            if colors.len() == 256 {
                return Ok(best);
            }
            colors.insert(*pixel, colors.len() as u8);
        }
    }
    let count = colors.len();
    let mut palette = vec![0; count * 3];
    let mut alpha = vec![255; count];
    for (color, &index) in &colors {
        palette[index as usize * 3..index as usize * 3 + 3].copy_from_slice(&color[..3]);
        alpha[index as usize] = color[3];
    }
    info.color_type = ColorType::Indexed;
    let depth = if count <= 2 {
        1
    } else if count <= 4 {
        2
    } else if count <= 16 {
        4
    } else {
        8
    };
    info.bit_depth = BitDepth::from_u8(depth).unwrap();
    info.palette = Some(Cow::Owned(palette));
    if !opaque {
        info.trns = Some(Cow::Owned(alpha));
    }
    let stride = (info.width as usize * depth as usize).div_ceil(8);
    let mut indices = vec![0; stride * info.height as usize];
    for (i, pixel) in pixels.iter().enumerate() {
        let x = i % info.width as usize;
        let y = i / info.width as usize;
        let bit = x * depth as usize;
        indices[y * stride + bit / 8] |= colors[pixel] << (8 - depth as usize - bit % 8);
    }
    encode(&indices, &info, &mut best)?;
    Ok(best)
}

/// Recompress an existing 8-bit RGB/RGBA PNG without changing a single pixel.
/// Legacy source bits and the canonical perceptual source chunk are both preserved.
pub fn optimize_png(bytes: &[u8]) -> Result<Vec<u8>, String> {
    let mut decoder = png::Decoder::new(Cursor::new(bytes));
    decoder.set_limits(png::Limits {
        bytes: 64_000_000 * 4,
    });
    let mut reader = decoder.read_info().map_err(|_| "Invalid PNG image")?;
    let info = reader.info().clone();
    if info.bit_depth != BitDepth::Eight
        || !matches!(info.color_type, ColorType::Rgb | ColorType::Rgba)
        || info.is_animated()
    {
        return Err("Optimize an original 8-bit RGB/RGBA PNG export; animated and 16-bit images are not converted".into());
    }
    if info.width as u64 * info.height as u64 > 64_000_000 {
        return Err("Image exceeds pixel budget".into());
    }
    // Preserve unusual transparency/background semantics by retaining the original.
    if info.trns.is_some() || info.bkgd.is_some() || info.sbit.is_some() {
        return Ok(bytes.to_vec());
    }
    let mut data = vec![0; reader.output_buffer_size()];
    let frame = reader
        .next_frame(&mut data)
        .map_err(|_| "PNG data is corrupt")?;
    data.truncate(frame.buffer_size());
    let rgba = if frame.color_type == ColorType::Rgb {
        data.as_chunks::<3>()
            .0
            .iter()
            .flat_map(|p| [p[0], p[1], p[2], 255])
            .collect()
    } else {
        data
    };
    let optimized = encode_pixels(&rgba, info)?;
    let optimized = crate::perceptual::preserve_magic_chunk(bytes, optimized)?;
    Ok(if optimized.len() < bytes.len() {
        optimized
    } else {
        bytes.to_vec()
    })
}

/// Share repeated legacy glyph paths without changing any outline or transform.
pub fn compact_svg(svg: &str) -> Result<String, String> {
    let document = roxmltree::Document::parse(svg).map_err(|_| "Invalid SVG artwork")?;
    let root = document.root_element();
    if root.tag_name().name() != "svg" {
        return Err("Invalid SVG artwork".into());
    }
    let paths: Vec<_> = document
        .descendants()
        .filter(|n| {
            n.has_tag_name("path")
                && n.has_attribute("data-glyph")
                && n.children().next().is_none()
                && svg[n.range()].starts_with("<path ")
        })
        .collect();
    let mut counts = BTreeMap::new();
    for node in &paths {
        if let Some(path) = node.attribute("d") {
            *counts.entry(path).or_insert(0_usize) += 1;
        }
    }
    let next = document
        .descendants()
        .filter_map(|n| {
            n.attribute("id")?
                .strip_prefix("glyph-")?
                .parse::<usize>()
                .ok()
        })
        .max()
        .map_or(0, |id| id + 1);
    let shared: BTreeMap<_, _> = counts
        .iter()
        .filter(|(_, count)| **count > 1)
        .enumerate()
        .map(|(i, (&path, _))| (path, next + i))
        .collect();
    if shared.is_empty() {
        return Ok(svg.to_string());
    }
    let mut replacements = Vec::new();
    let mut definitions = String::from("<defs>");
    for (&path, &id) in &shared {
        let escaped = path
            .replace('&', "&amp;")
            .replace('\'', "&apos;")
            .replace('<', "&lt;");
        definitions.push_str(&format!("<path id='glyph-{id}' d='{escaped}'/>"));
    }
    definitions.push_str("</defs>");
    for node in paths {
        let Some(&id) = node.attribute("d").and_then(|path| shared.get(path)) else {
            continue;
        };
        let range = node.range();
        let attribute = node.attribute_node("d").unwrap().range();
        let mut tag = svg[range.clone()].to_string();
        tag.replace_range(
            attribute.start - range.start..attribute.end - range.start,
            &format!("href='#glyph-{id}'"),
        );
        tag.replace_range(1..5, "use");
        replacements.push((range, tag));
    }
    let mut result = svg.to_string();
    for (range, tag) in replacements.into_iter().rev() {
        result.replace_range(range, &tag);
    }
    // Insert into the existing drawing, with no reformatting of the original geometry.
    result.insert_str(
        root.range().start
            + result[root.range().start..]
                .find('>')
                .ok_or("Invalid SVG artwork")?
            + 1,
        &definitions,
    );
    Ok(result)
}
