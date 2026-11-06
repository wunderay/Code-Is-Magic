//! Compact scripted magic SVG (format 3).
//!
//! The full drawing is dominated by geometry that is completely determined by a few
//! parameters: braided invocation channels (192 sampled points from one cubic curve),
//! inscription bands (every glyph placement follows from cell weights), collars
//! (repeated language templates), and name contours (one nibble per vertex). The
//! renderer tags each such subtree with its parameters (`data-m`). The canonical
//! export replaces each tagged subtree by a short macro element, and an embedded
//! script expands macros when the image is opened in a browser. Rust expands the same
//! macros with the exact functions the renderer used, so `expand(compact(svg))`
//! reproduces the renderer's markup for unquantized parameters byte for byte.
use crate::inscription::Outline;
use crate::profiles::{language_profiles, CollarOperation, LanguageProfile};
use crate::render::{circle, curve_point, poly};
use std::{collections::BTreeMap, f64::consts::PI, fmt::Write, sync::OnceLock};

pub(crate) const FORMAT: &str = "data-magic-format='3'";
const SCRIPT: &str = include_str!("magic.js");
/// Macro parameter precision. Braids and collars live in SVG pixel space; band weights
/// and sigil placement need a little more because they are multiplied by radii.
const BRAID_DECIMALS: usize = 2;
const RING_DECIMALS: usize = 3;
const BAND_DECIMALS: usize = 4;
const SIGIL_DECIMALS: usize = 4;
/// Regular polygons are often drawn in unit coordinates scaled by a seal radius.
const POLYGON_DECIMALS: usize = 5;
/// Glyph outlines are stored in units of this many font units, drawn with `scale`.
const GLYPH_GRID: f64 = 2.;
const GLYPH_SCALE: &str = "scale(2)";

fn number(token: Option<&str>) -> Result<f64, String> {
    token
        .ok_or("Truncated magic macro")?
        .parse::<f64>()
        .ok()
        .filter(|v| v.is_finite())
        .ok_or_else(|| "Invalid magic macro number".into())
}
fn index(token: Option<&str>) -> Result<usize, String> {
    token
        .ok_or("Truncated magic macro")?
        .parse()
        .map_err(|_| "Invalid magic macro index".into())
}
fn color(token: Option<&str>) -> Result<&str, String> {
    let token = token.ok_or("Truncated magic macro")?;
    (token.len() == 7
        && token.starts_with('#')
        && token[1..].bytes().all(|b| b.is_ascii_hexdigit()))
    .then_some(token)
    .ok_or_else(|| "Invalid magic macro color".into())
}

pub(crate) struct Braid<'a> {
    pub kind: &'a str,
    pub from: usize,
    pub to: usize,
    pub ink: &'a str,
    pub bg: &'a str,
    pub points: [[f64; 2]; 4],
    pub turns: f64,
    pub amplitude: f64,
    pub knots: Vec<f64>,
}
impl<'a> Braid<'a> {
    fn parse(payload: &'a str) -> Result<Self, String> {
        let mut t = payload.split_ascii_whitespace();
        let ink = color(t.next())?;
        let bg = color(t.next())?;
        let kind = t.next().ok_or("Truncated magic macro")?;
        if !kind.bytes().all(|b| b.is_ascii_lowercase() || b == b'_') {
            return Err("Invalid magic channel kind".into());
        }
        let from = index(t.next())?;
        let to = index(t.next())?;
        let turns = number(t.next())?;
        let amplitude = number(t.next())?;
        let mut points = [[0.; 2]; 4];
        for point in &mut points {
            *point = [number(t.next())?, number(t.next())?];
        }
        let knots = t.map(|k| number(Some(k))).collect::<Result<Vec<_>, _>>()?;
        if knots.len() > 16 || !(0. ..=64.).contains(&turns) {
            return Err("Invalid magic braid".into());
        }
        Ok(Self {
            kind,
            from,
            to,
            ink,
            bg,
            points,
            turns,
            amplitude,
            knots,
        })
    }
}
/// One braided invocation channel. This is the renderer's only drawing code for it.
pub(crate) fn write_braid(s: &mut String, b: &Braid<'_>) {
    let p = &b.points;
    let center = format!(
        "M {:.3} {:.3} C {:.3} {:.3}, {:.3} {:.3}, {:.3} {:.3}",
        p[0][0], p[0][1], p[1][0], p[1][1], p[2][0], p[2][1], p[3][0], p[3][1]
    );
    let (ink, bg) = (b.ink, b.bg);
    let _ = write!(
        s,
        "<g data-channel='{}' data-from='{}' data-to='{}' data-m='{ink} {bg} {} {} {} {} {}",
        b.kind, b.from, b.to, b.kind, b.from, b.to, b.turns, b.amplitude
    );
    for value in p.iter().flatten().chain(&b.knots) {
        let _ = write!(s, " {value}");
    }
    let _ = write!(s, "' fill='none' stroke='{ink}' stroke-linecap='round' stroke-linejoin='round'><path d='{center}' stroke-width='8' opacity='.055'/><path d='{center}' stroke-width='.65' stroke-dasharray='1 9' opacity='.42'/>");
    for strand in [-1., 1.] {
        let mut path = String::new();
        for i in 0..=96 {
            let t = i as f64 / 96.;
            let (point, tangent) = curve_point(p, t);
            let envelope = (PI * t).sin().max(0.).sqrt();
            let offset = strand * b.amplitude * (t * b.turns * 2. * PI).sin() * envelope;
            let _ = write!(
                path,
                "{} {:.3} {:.3} ",
                if i == 0 { "M" } else { "L" },
                point[0] - tangent[1] * offset,
                point[1] + tangent[0] * offset
            );
        }
        let _ = write!(
            s,
            "<path data-strand='1' d='{path}' stroke-width='1.05' opacity='.62'/>"
        );
    }
    // Engraved knots mark the braid; their spacing and forms are name-derived.
    let knots = b.knots.len();
    for (i, &size) in b.knots.iter().enumerate() {
        let t = (i as f64 + 1.) / (knots as f64 + 1.);
        let (point, tangent) = curve_point(p, t);
        let angle = tangent[1].atan2(tangent[0]) * 180. / PI;
        let _ = write!(s, "<g transform='translate({:.3} {:.3}) rotate({angle:.3})' fill='{bg}' stroke-width='.8' opacity='.85'>", point[0], point[1]);
        poly(s, 0., 0., size, 4, 1, 0.);
        if i % 2 == 0 {
            circle(s, 0., 0., size * 1.35, 0.7);
        }
        let _ = write!(
            s,
            "<path d='M {:.3} 0 L {:.3} 0 M 0 {:.3} L 0 {:.3}'/>",
            -size * 0.45,
            size * 0.45,
            -size * 0.45,
            size * 0.45
        );
        s.push_str("</g>");
    }
    let (source, _) = curve_point(p, 0.);
    let _ = write!(s, "<g fill='{bg}' stroke-width='.9'>");
    circle(s, source[0], source[1], 4., 0.75);
    circle(s, source[0], source[1], 1.6, 0.7);
    s.push_str("</g>");
    let (destination, tangent) = curve_point(p, 1.);
    let angle = tangent[1].atan2(tangent[0]) * 180. / PI;
    let _ = write!(s, "<g data-call-gate='1' transform='translate({:.3} {:.3}) rotate({angle:.3})' stroke-width='1' opacity='.92'>", destination[0], destination[1]);
    let _ = write!(s, "<g fill='{bg}'>");
    circle(s, 0., 0., 6.5, 0.9);
    circle(s, 0., 0., 3.7, 0.75);
    circle(s, 0., 0., 1.6, 0.9);
    s.push_str("</g></g></g>");
}

fn profile_index(id: &str) -> Option<usize> {
    language_profiles().iter().position(|p| p.id == id)
}
fn write_collar(s: &mut String, profile: &LanguageProfile) {
    for operation in &profile.collar {
        match operation {
            CollarOperation::Path { data } => {
                let _ = write!(s, "<path d='{data}'/>");
            }
            CollarOperation::Polygon {
                cx,
                cy,
                r,
                n,
                step,
                rotation_degrees,
            } => poly(s, *cx, *cy, *r, *n, *step, rotation_degrees * PI / 180.),
        }
    }
}
/// The calm outer collar: three rings and the language's repeated ornament.
pub(crate) fn write_ring(
    s: &mut String,
    x: f64,
    y: f64,
    r: f64,
    rotation: u64,
    profile: &LanguageProfile,
) {
    let _ = write!(
        s,
        "<g data-ring='1' data-m='{x} {y} {r} {rotation} {}' stroke-width='{:.3}'>",
        profile.id,
        (r * 0.004).max(0.85)
    );
    circle(s, x, y, r, 0.92);
    circle(s, x, y, r * 0.93, 0.65);
    circle(s, x, y, r * 0.73, 0.72);
    let rotation = rotation as f64 * PI / 180.;
    for i in 0..profile.repeats {
        let angle = rotation + i as f64 * 2. * PI / profile.repeats as f64;
        let degrees = angle * 180. / PI;
        let _ = write!(s, "<g transform='translate({x} {y}) rotate({degrees:.3}) scale({r})' stroke-width='.007'>");
        write_collar(s, profile);
        s.push_str("</g>");
    }
    s.push_str("</g>");
}
fn expand_ring(s: &mut String, payload: &str) -> Result<(), String> {
    let mut t = payload.split_ascii_whitespace();
    let (x, y, r) = (number(t.next())?, number(t.next())?, number(t.next())?);
    let rotation = index(t.next())? as u64 % 360;
    let profile = t
        .next()
        .and_then(profile_index)
        .ok_or("Unknown collar language")?;
    write_ring(s, x, y, r, rotation, &language_profiles()[profile]);
    Ok(())
}

pub(crate) struct Cell {
    pub glyphs: Vec<usize>,
    pub height: f64,
    pub lean: f64,
    pub weight: f64,
}
pub(crate) struct Band {
    pub x: f64,
    pub y: f64,
    pub radius: f64,
    pub inner: f64,
    pub outer: f64,
    pub style: u64,
    pub rotation: u64,
    pub cells: Vec<Cell>,
}
pub(crate) fn glyphs() -> &'static [Outline] {
    static GLYPHS: OnceLock<Vec<Outline>> = OnceLock::new();
    GLYPHS.get_or_init(|| {
        let outlines: BTreeMap<String, Outline> =
            serde_json::from_str(include_str!("glyphs.json")).expect("bundled measured glyphs");
        outlines
            .into_values()
            .enumerate()
            .map(|(id, mut outline)| {
                outline.id = id;
                outline
            })
            .collect()
    })
}
/// Fit the actual outline, preserving aspect ratio and bounding its rotated box.
fn fit_glyph(s: &mut String, glyph: &Outline, width: f64, height: f64, offset: f64, lean: f64) {
    let angle = lean * PI / 180.;
    let (cos, sin) = (angle.cos().abs(), angle.sin().abs());
    let rotated_width = glyph.width * cos + glyph.height * sin;
    let rotated_height = glyph.height * cos + glyph.width * sin;
    let scale = (width / rotated_width).min(height / rotated_height);
    let actual_width = rotated_width * scale;
    let actual_height = rotated_height * scale;
    let _ = write!(s, "<use data-glyph='1' data-family='{}' data-width='{actual_width:.6}' data-height='{actual_height:.6}' data-offset='{offset:.6}' href='#glyph-{}' transform='translate(0 {offset:.6}) rotate({lean:.6}) scale({scale:.10} -{scale:.10}) translate({:.6} {:.6})'/>", glyph.family, glyph.id, -glyph.cx, -glyph.cy);
}
/// One annular inscription band of measured proportional glyph cells.
pub(crate) fn write_band(s: &mut String, band: &Band) {
    let (x, y, radius, inner, outer) = (band.x, band.y, band.radius, band.inner, band.outer);
    let glyphs = glyphs();
    let middle = radius * (inner + outer) / 2.;
    let band_height = radius * (outer - inner);
    let target_height = band_height * 0.81;
    let total = band.cells.iter().map(|c| c.weight).sum::<f64>();
    let _ = write!(s, "<g data-inscription-band='2' data-inner='{:.6}' data-outer='{:.6}' data-middle='{middle:.6}' data-style='{}' data-m='{x} {y} {radius} {inner} {outer} {} {}", radius * inner, radius * outer, band.style, band.style, band.rotation);
    for cell in &band.cells {
        let _ = write!(s, ",{} {} {}", cell.weight, cell.height, cell.lean);
        for glyph in &cell.glyphs {
            let _ = write!(s, " {glyph}");
        }
    }
    let _ = write!(
        s,
        "' transform='translate({x} {y}) rotate({})' opacity='.94'>",
        band.rotation
    );
    let mut angle = 0.;
    for cell in &band.cells {
        let sector = cell.weight * 2. * PI / total;
        // A chord at the inner boundary reserves angular space for the full glyph.
        let width = 2. * radius * inner * (sector / 2.).sin() * 0.84;
        let height = (target_height * cell.height).min(band_height * 0.86);
        // Constrain the corners to the annulus, not just the baseline.
        let safe_width = 2.
            * ((radius * outer).powi(2) - (middle + height / 2.).powi(2))
                .max(0.)
                .sqrt();
        let width = width.min(safe_width * 0.97);
        let degrees = (angle + sector / 2.) * 180. / PI;
        let _ = write!(s, "<g data-cell='1' data-angle='{degrees:.6}' data-sector='{sector:.10}' transform='rotate({degrees:.6}) translate(0 -{middle:.6})'>");
        for (i, &glyph) in cell.glyphs.iter().enumerate() {
            let (h, offset) = if cell.glyphs.len() == 2 {
                (height * 0.44, height * if i == 0 { -0.27 } else { 0.27 })
            } else {
                (height, 0.)
            };
            fit_glyph(s, &glyphs[glyph], width, h, offset, cell.lean);
        }
        s.push_str("</g>");
        angle += sector;
    }
    s.push_str("</g>");
}
/// Cell weights follow from glyph proportions, so the compact band stores only each
/// cell's height (steps of 0.002 above 0.9), lean (fifths of a degree) and glyph ids; at
/// inscription scale both steps move a glyph corner by well under a tenth of a pixel.
fn compact_band(payload: &str) -> Result<String, String> {
    let mut parts = payload.split(',');
    let mut out = quantize(parts.next().unwrap_or_default(), BAND_DECIMALS);
    for part in parts {
        let mut t = part.split_ascii_whitespace();
        let _weight = number(t.next())?;
        let height = ((number(t.next())? - 0.9) * 500.).round() as i64;
        let lean = (number(t.next())? * 5.).round() as i64;
        let _ = write!(out, ",{height} {lean}");
        for glyph in t {
            let _ = write!(out, " {}", index(Some(glyph))?);
        }
    }
    Ok(out)
}
pub(crate) fn cell_weight(glyphs: &[usize], height: f64, style: u64, i: usize) -> f64 {
    let outlines = self::glyphs();
    let width = glyphs
        .iter()
        .map(|&g| outlines[g].width / outlines[g].height)
        .reduce(f64::max)
        .unwrap_or(1.);
    width.clamp(0.24, 1.9) * height * if glyphs.len() == 2 { 0.46 } else { 1. }
        + 0.19
        + if style == 2 && i.is_multiple_of(4) {
            0.40
        } else {
            0.
        }
}
fn expand_band(s: &mut String, payload: &str) -> Result<(), String> {
    let mut parts = payload.split(',');
    let mut t = parts.next().unwrap_or_default().split_ascii_whitespace();
    let (x, y, radius) = (number(t.next())?, number(t.next())?, number(t.next())?);
    let (inner, outer) = (number(t.next())?, number(t.next())?);
    let (style, rotation) = (index(t.next())? as u64, index(t.next())? as u64);
    let mut cells = Vec::new();
    for (i, part) in parts.enumerate() {
        let mut t = part.split_ascii_whitespace();
        let height = 0.9 + number(t.next())? / 500.;
        let lean = number(t.next())? / 5.;
        let glyphs = t.map(|g| index(Some(g))).collect::<Result<Vec<_>, _>>()?;
        if !(1..=2).contains(&glyphs.len()) || glyphs.iter().any(|&g| g >= self::glyphs().len()) {
            return Err("Invalid magic inscription cell".into());
        }
        cells.push(Cell {
            weight: cell_weight(&glyphs, height, style, i),
            glyphs,
            height,
            lean,
        });
    }
    if cells.is_empty() || cells.len() > 256 || !(radius > 0. && inner < outer) {
        return Err("Invalid magic inscription band".into());
    }
    write_band(
        s,
        &Band {
            x,
            y,
            radius,
            inner,
            outer,
            style,
            rotation,
            cells,
        },
    );
    Ok(())
}
fn expand_polygon(s: &mut String, payload: &str) -> Result<(), String> {
    let mut t = payload.split_ascii_whitespace();
    let (x, y, r) = (number(t.next())?, number(t.next())?, number(t.next())?);
    let (n, step) = (index(t.next())?, index(t.next())?);
    if !(1..=64).contains(&n) {
        return Err("Invalid magic polygon".into());
    }
    poly(s, x, y, r, n, step, number(t.next())?);
    Ok(())
}

/// The exact reversible name contour: one radial vertex per frame nibble.
pub(crate) fn write_sigil(s: &mut String, entity: usize, x: f64, y: f64, r: f64, nibbles: &[u8]) {
    let length = nibbles.len();
    let core = r * 0.52;
    let _ = write!(
        s,
        "<polygon data-name-sigil='1' data-entity='{entity}' data-seal-radius='{r}' data-m='{entity} {x} {y} {r} "
    );
    for &nibble in nibbles {
        s.push(char::from_digit(nibble as u32, 16).unwrap());
    }
    s.push_str("' points='");
    for (i, &nibble) in nibbles.iter().enumerate() {
        let angle = -PI / 2. + i as f64 * 2. * PI / length as f64;
        let radius = 0.72 + nibble as f64 * 0.014;
        let _ = write!(
            s,
            "{}{:.6},{:.6}",
            if i == 0 { "" } else { " " },
            (radius * angle.cos() * 1_000_000.).round() / 1_000_000.,
            (radius * angle.sin() * 1_000_000.).round() / 1_000_000.
        );
    }
    let _ = write!(s, "' transform='translate({x} {y}) scale({core})' stroke-width='.007' opacity='.65' stroke-linejoin='round'/>");
}
pub(crate) fn sigil_nibbles(points: &[[f64; 2]]) -> Vec<u8> {
    points
        .iter()
        .map(|p| ((p[0].hypot(p[1]) - 0.72) / 0.014).round().clamp(0., 15.) as u8)
        .collect()
}
fn expand_sigil(s: &mut String, payload: &str) -> Result<(), String> {
    let mut t = payload.split_ascii_whitespace();
    let entity = index(t.next())?;
    let (x, y, r) = (number(t.next())?, number(t.next())?, number(t.next())?);
    let nibbles = t
        .next()
        .ok_or("Truncated magic macro")?
        .chars()
        .map(|c| c.to_digit(16).map(|d| d as u8))
        .collect::<Option<Vec<_>>>()
        .ok_or("Invalid name contour")?;
    write_sigil(s, entity, x, y, r, &nibbles);
    Ok(())
}

/// Round decimal tokens only. Colors, indices and nibble strings are never numbers here.
fn quantize(payload: &str, decimals: usize) -> String {
    let mut out = String::with_capacity(payload.len());
    let mut token = String::new();
    let flush = |token: &mut String, out: &mut String| {
        match token.parse::<f64>() {
            Ok(value) if token.contains('.') && value.is_finite() => {
                let mut text = format!("{value:.decimals$}");
                while text.contains('.') && text.ends_with('0') {
                    text.pop();
                }
                if text.ends_with('.') {
                    text.pop();
                }
                out.push_str(if text == "-0" { "0" } else { &text });
            }
            _ => out.push_str(token),
        }
        token.clear();
    };
    for ch in payload.chars() {
        if ch == ' ' || ch == ',' {
            flush(&mut token, &mut out);
            out.push(ch);
        } else {
            token.push(ch);
        }
    }
    flush(&mut token, &mut out);
    out
}
fn tag(node: &roxmltree::Node<'_, '_>) -> Option<(&'static str, usize)> {
    node.attribute("data-m")?;
    Some(if node.has_attribute("data-channel") {
        ("B", BRAID_DECIMALS)
    } else if node.has_attribute("data-ring") {
        ("O", RING_DECIMALS)
    } else if node.has_attribute("data-inscription-band") {
        ("I", BAND_DECIMALS)
    } else if node.has_attribute("data-name-sigil") {
        ("N", SIGIL_DECIMALS)
    } else if node.has_tag_name("polyline") {
        ("Y", POLYGON_DECIMALS)
    } else {
        return None;
    })
}

fn push_number(out: &mut String, value: f64) {
    let value = if value == 0. { 0. } else { value };
    let text = format!("{value}");
    let glued = text.starts_with('-') || out.ends_with(|c: char| c.is_ascii_alphabetic());
    if !glued && !out.is_empty() {
        out.push(' ');
    }
    out.push_str(&text);
}
/// Rewrite a glyph outline with relative commands on a grid of `GLYPH_GRID` font units. The
/// grid is tracked in absolute rounded coordinates, so rounding never accumulates;
/// at inscription scale one font unit is roughly a sixtieth of a pixel, so the 2-unit grid
/// moves an outline point by at most about a sixtieth of a pixel.
fn relative_path(d: &str, grid: f64) -> Result<String, String> {
    let bytes = d.as_bytes();
    let mut tokens: Vec<Result<char, f64>> = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        let c = bytes[i] as char;
        if c.is_ascii_alphabetic() {
            tokens.push(Ok(c));
            i += 1;
        } else if c == '-' || c == '+' || c == '.' || c.is_ascii_digit() {
            let start = i;
            i += 1;
            let mut dot = c == '.';
            while i < bytes.len() {
                let b = bytes[i] as char;
                if b.is_ascii_digit() || (b == '.' && !dot) {
                    dot |= b == '.';
                    i += 1;
                } else if (b == 'e' || b == 'E') && i + 1 < bytes.len() {
                    i += 1;
                    if matches!(bytes[i], b'-' | b'+') {
                        i += 1;
                    }
                } else {
                    break;
                }
            }
            tokens.push(Err(d[start..i]
                .parse()
                .map_err(|_| "Invalid glyph path")?));
        } else {
            i += 1;
        }
    }
    let mut out = String::new();
    let (mut true_x, mut true_y, mut round_x, mut round_y) = (0., 0., 0., 0.);
    let (mut start_true, mut start_round) = ([0., 0.], [0., 0.]);
    let mut t = tokens.into_iter().peekable();
    let mut command = 'M';
    while t.peek().is_some() {
        if let Some(Ok(c)) = t.peek() {
            command = *c;
            t.next();
            if matches!(command, 'Z' | 'z') {
                out.push('z');
                (true_x, true_y) = (start_true[0], start_true[1]);
                (round_x, round_y) = (start_round[0], start_round[1]);
                continue;
            }
            out.push(command.to_ascii_lowercase());
        } else {
            // Implicit repeats of the previous command; a moveto repeats as lineto.
            if command == 'M' {
                command = 'L';
            } else if command == 'm' {
                command = 'l';
            }
        }
        let relative = command.is_ascii_lowercase();
        let upper = command.to_ascii_uppercase();
        let mut next = || match t.next() {
            Some(Err(v)) => Ok(v),
            _ => Err("Invalid glyph path".to_string()),
        };
        let point = |next: &mut dyn FnMut() -> Result<f64, String>,
                     out: &mut String,
                     base: (f64, f64, f64, f64)|
         -> Result<(f64, f64), String> {
            let (x, y) = (next()?, next()?);
            let (ax, ay) = if relative {
                (base.0 + x, base.1 + y)
            } else {
                (x, y)
            };
            push_number(out, (ax / grid).round() - base.2);
            push_number(out, (ay / grid).round() - base.3);
            Ok((ax, ay))
        };
        let base = (true_x, true_y, round_x, round_y);
        let end = match upper {
            'M' | 'L' => point(&mut next, &mut out, base)?,
            'C' => {
                point(&mut next, &mut out, base)?;
                point(&mut next, &mut out, base)?;
                point(&mut next, &mut out, base)?
            }
            'Q' => {
                point(&mut next, &mut out, base)?;
                point(&mut next, &mut out, base)?
            }
            'H' => {
                let x = next()?;
                let ax = if relative { true_x + x } else { x };
                push_number(&mut out, (ax / grid).round() - round_x);
                (ax, true_y)
            }
            'V' => {
                let y = next()?;
                let ay = if relative { true_y + y } else { y };
                push_number(&mut out, (ay / grid).round() - round_y);
                (true_x, ay)
            }
            'A' => {
                for _ in 0..2 {
                    let radius = next()?;
                    push_number(&mut out, (radius / grid).round().max(1.));
                }
                for _ in 0..3 {
                    let v = next()?;
                    push_number(&mut out, v);
                }
                point(&mut next, &mut out, base)?
            }
            _ => return Err("Unsupported glyph path command".into()),
        };
        (true_x, true_y) = end;
        (round_x, round_y) = ((end.0 / grid).round(), (end.1 / grid).round());
        if upper == 'M' {
            start_true = [true_x, true_y];
            start_round = [round_x, round_y];
        }
    }
    Ok(out)
}

/// Replace every parameterized subtree by its macro and add what the embedded
/// expander needs: glyph metrics, collar templates and the script itself.
pub(crate) fn compact(svg: &str) -> Result<String, String> {
    if svg.contains(FORMAT) {
        return Ok(svg.into());
    }
    let document = roxmltree::Document::parse(svg).map_err(|_| "Invalid SVG artwork")?;
    let root = document.root_element();
    if !root.has_tag_name("svg") || svg.contains("<script") {
        return Err("Invalid SVG artwork".into());
    }
    let glyphs = glyphs();
    let mut edits: Vec<(std::ops::Range<usize>, String)> = Vec::new();
    let mut languages = std::collections::BTreeSet::new();
    let mut skip_until = 0;
    for node in document.descendants().filter(|n| n.is_element()) {
        if node.range().start < skip_until {
            continue;
        }
        if let Some((name, decimals)) = tag(&node) {
            let payload = node.attribute("data-m").unwrap();
            if name == "O" {
                if let Some(id) = payload.split_ascii_whitespace().nth(4) {
                    languages.insert(profile_index(id).ok_or("Unknown collar language")?);
                }
            }
            let payload = if name == "I" {
                compact_band(payload)?
            } else {
                quantize(payload, decimals)
            };
            skip_until = node.range().end;
            edits.push((node.range(), format!("<{name} d='{payload}'/>")));
        } else if node.has_tag_name("path")
            && node
                .parent_element()
                .is_some_and(|p| p.has_tag_name("defs"))
        {
            let Some(id) = node
                .attribute("id")
                .and_then(|id| id.strip_prefix("glyph-")?.parse::<usize>().ok())
            else {
                continue;
            };
            let gridded = node.attribute("transform") == Some(GLYPH_SCALE);
            let d = node.attribute("d").ok_or("Invalid glyph definition")?;
            let d = relative_path(d, if gridded { 1. } else { GLYPH_GRID })?;
            let k = glyphs.get(id).map_or(String::new(), |g| {
                format!(" k='{} {} {} {}'", g.cx, g.cy, g.width, g.height)
            });
            edits.push((
                node.range(),
                format!("<path id='glyph-{id}' transform='{GLYPH_SCALE}' d='{d}'{k}/>"),
            ));
        }
    }
    let mut out = svg.to_string();
    for (range, text) in edits.into_iter().rev() {
        out.replace_range(range, &text);
    }
    let mut templates = String::new();
    for &language in &languages {
        let profile = &language_profiles()[language];
        let _ = write!(
            templates,
            "<g id='L{language}' l='{}' n='{}' stroke-width='.007'>",
            profile.id, profile.repeats
        );
        write_collar(&mut templates, profile);
        templates.push_str("</g>");
    }
    match out.find("</defs>") {
        Some(at) => out.insert_str(at, &templates),
        None => {
            let at = out.find('>').ok_or("Invalid SVG artwork")? + 1;
            out.insert_str(at, &format!("<defs>{templates}</defs>"));
        }
    }
    let end = out.rfind("</svg>").ok_or("Invalid SVG artwork")?;
    out.insert_str(end, &format!("<script><![CDATA[{SCRIPT}]]></script>"));
    let start = out.find("<svg ").ok_or("Invalid SVG artwork")? + 5;
    out.insert_str(start, &format!("{FORMAT} "));
    Ok(out)
}

/// Expand a compact magic SVG back to the renderer's static markup. Other SVG passes
/// through unchanged.
pub fn expand_svg(svg: &str) -> Result<String, String> {
    if !svg.contains(FORMAT) {
        return Ok(svg.into());
    }
    let document = roxmltree::Document::parse(svg).map_err(|_| "Invalid magic SVG")?;
    let root = document.root_element();
    if !root.has_tag_name("svg") {
        return Err("Invalid magic SVG".into());
    }
    let mut edits: Vec<(std::ops::Range<usize>, String)> = Vec::new();
    for node in document.descendants().filter(|n| n.is_element()) {
        let name = node.tag_name().name();
        let mut out = String::new();
        match name {
            "B" | "O" | "I" | "N" | "Y" => {
                let payload = node.attribute("d").ok_or("Empty magic macro")?;
                match name {
                    "B" => write_braid(&mut out, &Braid::parse(payload)?),
                    "O" => expand_ring(&mut out, payload)?,
                    "I" => expand_band(&mut out, payload)?,
                    "Y" => expand_polygon(&mut out, payload)?,
                    _ => expand_sigil(&mut out, payload)?,
                }
                edits.push((node.range(), out));
            }
            "script" => edits.push((node.range(), out)),
            "g" if node.attribute("id").is_some_and(|id| id.starts_with('L'))
                && node
                    .parent_element()
                    .is_some_and(|p| p.has_tag_name("defs")) =>
            {
                edits.push((node.range(), out))
            }
            "path" => {
                if let Some(k) = node.attribute_node("k") {
                    let range = k.range();
                    edits.push((range.start - 1..range.end, out));
                }
            }
            _ => {}
        }
    }
    edits.sort_by_key(|(range, _)| range.start);
    let mut result = svg.to_string();
    let mut last = usize::MAX;
    for (range, text) in edits.into_iter().rev() {
        if range.end > last {
            return Err("Overlapping magic macros".into());
        }
        last = range.start;
        result.replace_range(range, &text);
    }
    Ok(result.replacen(&format!("{FORMAT} "), "", 1))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{analyze, render_with_key, MagicKey, SourceFile};

    fn drawing(key: Option<&MagicKey>) -> String {
        let mut files = Vec::new();
        for i in 0..24 {
            let body = "    let x = 1;\n".repeat(1 + i * 3);
            let next = (i + 1) % 24;
            files.push(SourceFile {
                path: format!("m{i}.{}", ["rs", "py"][i % 2]),
                content: if i % 2 == 0 {
                    format!(
                        "fn s{i}() {{\n{body}    s{next}();\n    s{}();\n}}\n",
                        (i + 2) % 24
                    )
                } else {
                    format!(
                        "def s{i}():\n{}    return s{}()\n",
                        body.replace("let ", ""),
                        (i + 2) % 24
                    )
                },
            });
        }
        let analysis = analyze(&files);
        render_with_key(&analysis, "t", 4, "gold", false, true, key)
            .unwrap()
            .svg
    }

    #[test]
    fn unquantized_macros_reproduce_renderer_markup_exactly() {
        let svg = drawing(None);
        let document = roxmltree::Document::parse(&svg).unwrap();
        let mut checked = BTreeMap::new();
        for node in document.descendants().filter(|n| n.is_element()) {
            let Some((name, _)) = tag(&node) else {
                continue;
            };
            let payload = node.attribute("data-m").unwrap();
            let mut out = String::new();
            match name {
                "B" => write_braid(&mut out, &Braid::parse(payload).unwrap()),
                "O" => expand_ring(&mut out, payload).unwrap(),
                "N" => expand_sigil(&mut out, payload).unwrap(),
                "Y" => expand_polygon(&mut out, payload).unwrap(),
                _ => continue,
            }
            assert_eq!(out, &svg[node.range()], "{name} macro diverged");
            *checked.entry(name).or_insert(0) += 1;
        }
        assert_eq!(checked.len(), 4, "{checked:?}");
    }

    #[test]
    fn compact_artwork_is_small_scripted_idempotent_and_name_exact() {
        for key in [None, Some(MagicKey::from_hex(&"07".repeat(32)).unwrap())] {
            let full = drawing(key.as_ref());
            let compact = crate::magic_svg(&full).unwrap();
            assert!(compact.contains(FORMAT) && compact.contains("<script>"));
            assert!(!compact.contains("data-strand") && !compact.contains("data-cell"));
            assert!(
                compact.len() * 4 < full.len(),
                "{} vs {}",
                compact.len(),
                full.len()
            );
            assert_eq!(crate::magic_svg(&compact).unwrap(), compact);
            let shown = expand_svg(&compact).unwrap();
            assert!(!shown.contains("<script") && !shown.contains(FORMAT));
            assert_eq!(crate::magic_svg(&shown).unwrap(), compact);
            assert_eq!(expand_svg(&full).unwrap(), full);
            assert_eq!(
                crate::decode_svg_names_with_key(&compact, key.as_ref()).unwrap(),
                crate::decode_svg_names_with_key(&full, key.as_ref()).unwrap()
            );
            // Same element structure as the renderer's drawing.
            for marker in [
                "data-cell='1'",
                "data-strand='1'",
                "data-name-sigil='1'",
                "<use ",
            ] {
                assert_eq!(shown.matches(marker).count(), full.matches(marker).count());
            }
        }
    }

    #[test]
    fn glyph_paths_become_relative_on_the_grid() {
        let absolute =
            "M-23 -410L-23 410H23V-410ZM10.4 5Q20 30 40 5C1 2 3 4 5 6A10 12 30 0 1 50 60L1 1 2 2z";
        let relative = relative_path(absolute, 1.).unwrap();
        assert_eq!(&relative[..21], "m-23-410l0 820h46v-82");
        // Relative input describes the same outline and is a fixed point.
        assert_eq!(relative_path(&relative, 1.).unwrap(), relative);
        assert_eq!(relative_path("m1 1 2 2l3 3", 1.).unwrap(), "m1 1 2 2l3 3");
        assert_eq!(relative_path("M10 10 20 20", 2.).unwrap(), "m5 5 5 5");
        assert!(relative_path("M1", 1.).is_err());
    }

    #[test]
    fn macro_quantization_never_touches_colors_or_nibbles() {
        assert_eq!(
            quantize("#1e5a3b 1e5 0.123456,-0.00001 c71e5", 2),
            "#1e5a3b 1e5 0.12,0 c71e5"
        );
        assert!(expand_svg("<svg data-magic-format='3' xmlns='http://www.w3.org/2000/svg'><B d='#zz0000 #000000 calls 0 1 3 4 0 0 1 1 2 2 3 3'/></svg>").is_err());
        assert!(expand_svg("<svg data-magic-format='3' xmlns='http://www.w3.org/2000/svg'><I d='0 0 10 0.7 0.9 0 0,1 1 99999'/></svg>").is_err());
    }
}
