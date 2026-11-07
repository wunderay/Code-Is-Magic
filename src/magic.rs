//! Compact scripted magic SVG (format 3).
//!
//! The full drawing is dominated by geometry that is completely determined by a few
//! parameters: braided invocation channels (192 sampled points from one cubic curve),
//! inscription bands (every glyph placement follows from cell weights), ward rings
//! (repeated language rune templates), and name contours (one nibble per vertex). The
//! renderer tags each such subtree with its parameters (`data-m`). The canonical
//! export replaces each tagged subtree by a short macro element, and an embedded
//! script expands macros when the image is opened in a browser. Rust expands the same
//! macros with the exact functions the renderer used, so `expand(compact(svg))`
//! reproduces the renderer's markup for unquantized parameters byte for byte.
use crate::inscription::Outline;
use crate::profiles::{language_profiles, LanguageProfile, WardRune};
use crate::render::{circle, curve_point, poly};
use std::{collections::BTreeMap, f64::consts::PI, fmt::Write, sync::OnceLock};

pub(crate) const FORMAT: &str = "data-magic-format='4'";
/// Every compact magic SVG names its format here; only `FORMAT` can be unfolded.
const FORMAT_MARK: &str = "data-magic-format=";

/// Reject compact artwork written in another (older) macro dialect instead of
/// silently leaving its macros unexpanded.
fn guard_format(vellum: &str) -> Result<bool, String> {
    match (vellum.contains(FORMAT_MARK), vellum.contains(FORMAT)) {
        (false, _) => Ok(false),
        (true, true) => Ok(true),
        (true, false) => Err(
            "Unsupported magic format: this compact image was written by another Code Is Magic version"
                .into(),
        ),
    }
}
const SCRIPT: &str = include_str!("magic.js");
/// Macro parameter precision. Braids and ward rings live in SVG pixel space; band weights
/// and sigil placement need a little more because they are multiplied by radii.
const BRAID_DECIMALS: usize = 2;
const WARD_DECIMALS: usize = 3;
const BAND_DECIMALS: usize = 4;
const SIGIL_DECIMALS: usize = 4;
/// Regular polygons are often drawn in unit coordinates scaled by a seal radius.
const POLYGON_DECIMALS: usize = 5;
/// Glyph outlines are stored in units of this many font units, drawn with `scale`.
const GLYPH_GRID: f64 = 2.;
const GLYPH_SCALE: &str = "scale(2)";

fn divine_number(word: Option<&str>) -> Result<f64, String> {
    word.ok_or("Truncated magic macro")?
        .parse::<f64>()
        .ok()
        .filter(|rune| rune.is_finite())
        .ok_or_else(|| "Invalid magic macro number".into())
}
fn divine_index(word: Option<&str>) -> Result<usize, String> {
    word.ok_or("Truncated magic macro")?
        .parse()
        .map_err(|_| "Invalid magic macro index".into())
}
fn divine_color(word: Option<&str>) -> Result<&str, String> {
    let word = word.ok_or("Truncated magic macro")?;
    (word.len() == 7
        && word.starts_with('#')
        && word[1..].bytes().all(|byte| byte.is_ascii_hexdigit()))
    .then_some(word)
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
    fn parse(spell: &'a str) -> Result<Self, String> {
        let mut words = spell.split_ascii_whitespace();
        let ink = divine_color(words.next())?;
        let bg = divine_color(words.next())?;
        let kind = words.next().ok_or("Truncated magic macro")?;
        if !kind
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte == b'_')
        {
            return Err("Invalid magic channel kind".into());
        }
        let from = divine_index(words.next())?;
        let to = divine_index(words.next())?;
        let turns = divine_number(words.next())?;
        let amplitude = divine_number(words.next())?;
        let mut points = [[0.; 2]; 4];
        for point in &mut points {
            *point = [divine_number(words.next())?, divine_number(words.next())?];
        }
        let knots = words
            .map(|knot| divine_number(Some(knot)))
            .collect::<Result<Vec<_>, _>>()?;
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
pub(crate) fn write_braid(scroll: &mut String, braid: &Braid<'_>) {
    let anchors = &braid.points;
    let ley_line = format!(
        "M {:.3} {:.3} C {:.3} {:.3}, {:.3} {:.3}, {:.3} {:.3}",
        anchors[0][0],
        anchors[0][1],
        anchors[1][0],
        anchors[1][1],
        anchors[2][0],
        anchors[2][1],
        anchors[3][0],
        anchors[3][1]
    );
    let (ink, bg) = (braid.ink, braid.bg);
    let _ = write!(
        scroll,
        "<g data-channel='{}' data-from='{}' data-to='{}' data-m='{ink} {bg} {} {} {} {} {}",
        braid.kind,
        braid.from,
        braid.to,
        braid.kind,
        braid.from,
        braid.to,
        braid.turns,
        braid.amplitude
    );
    for value in anchors.iter().flatten().chain(&braid.knots) {
        let _ = write!(scroll, " {value}");
    }
    let _ = write!(scroll, "' fill='none' stroke='{ink}' stroke-linecap='round' stroke-linejoin='round'><path d='{ley_line}' stroke-width='8' opacity='.055'/><path d='{ley_line}' stroke-width='.65' stroke-dasharray='1 9' opacity='.42'/>");
    for strand in [-1., 1.] {
        let mut trail = String::new();
        for step in 0..=96 {
            let progress = step as f64 / 96.;
            let (spot, bearing) = curve_point(anchors, progress);
            let swell = (PI * progress).sin().max(0.).sqrt();
            let sway = strand * braid.amplitude * (progress * braid.turns * 2. * PI).sin() * swell;
            let _ = write!(
                trail,
                "{} {:.3} {:.3} ",
                if step == 0 { "M" } else { "L" },
                spot[0] - bearing[1] * sway,
                spot[1] + bearing[0] * sway
            );
        }
        let _ = write!(
            scroll,
            "<path data-strand='1' d='{trail}' stroke-width='1.05' opacity='.62'/>"
        );
    }
    // Engraved knots mark the braid; their spacing and forms are name-derived.
    let knot_count = braid.knots.len();
    for (nth_knot, &size) in braid.knots.iter().enumerate() {
        let progress = (nth_knot as f64 + 1.) / (knot_count as f64 + 1.);
        let (spot, bearing) = curve_point(anchors, progress);
        let angle = bearing[1].atan2(bearing[0]) * 180. / PI;
        let _ = write!(scroll, "<g transform='translate({:.3} {:.3}) rotate({angle:.3})' fill='{bg}' stroke-width='.8' opacity='.85'>", spot[0], spot[1]);
        poly(scroll, 0., 0., size, 4, 1, 0.);
        if nth_knot % 2 == 0 {
            circle(scroll, 0., 0., size * 1.35, 0.7);
        }
        let _ = write!(
            scroll,
            "<path d='M {:.3} 0 L {:.3} 0 M 0 {:.3} L 0 {:.3}'/>",
            -size * 0.45,
            size * 0.45,
            -size * 0.45,
            size * 0.45
        );
        scroll.push_str("</g>");
    }
    let (wellspring, _) = curve_point(anchors, 0.);
    let _ = write!(scroll, "<g fill='{bg}' stroke-width='.9'>");
    circle(scroll, wellspring[0], wellspring[1], 4., 0.75);
    circle(scroll, wellspring[0], wellspring[1], 1.6, 0.7);
    scroll.push_str("</g>");
    let (sanctum, bearing) = curve_point(anchors, 1.);
    let angle = bearing[1].atan2(bearing[0]) * 180. / PI;
    let _ = write!(scroll, "<g data-call-gate='1' transform='translate({:.3} {:.3}) rotate({angle:.3})' stroke-width='1' opacity='.92'>", sanctum[0], sanctum[1]);
    let _ = write!(scroll, "<g fill='{bg}'>");
    circle(scroll, 0., 0., 6.5, 0.9);
    circle(scroll, 0., 0., 3.7, 0.75);
    circle(scroll, 0., 0., 1.6, 0.9);
    scroll.push_str("</g></g></g>");
}

fn divine_tongue(tongue_id: &str) -> Option<usize> {
    language_profiles()
        .iter()
        .position(|tongue| tongue.id == tongue_id)
}
fn inscribe_ward_runes(scroll: &mut String, tongue: &LanguageProfile) {
    for operation in &tongue.ward_ring {
        match operation {
            WardRune::Path { data } => {
                let _ = write!(scroll, "<path d='{data}'/>");
            }
            WardRune::Polygon {
                cx: hub_x,
                cy: hub_y,
                r: radius,
                n: sides,
                step: skip,
                rotation_degrees,
            } => poly(
                scroll,
                *hub_x,
                *hub_y,
                *radius,
                *sides,
                *skip,
                rotation_degrees * PI / 180.,
            ),
        }
    }
}
/// The calm ward ring: three rings and the language's repeated ward runes.
pub(crate) fn raise_ward_ring(
    scroll: &mut String,
    hub_x: f64,
    hub_y: f64,
    radius: f64,
    rotation: u64,
    tongue: &LanguageProfile,
) {
    let _ = write!(
        scroll,
        "<g data-ward-ring='1' data-m='{hub_x} {hub_y} {radius} {rotation} {}' stroke-width='{:.3}'>",
        tongue.id,
        (radius * 0.004).max(0.85)
    );
    circle(scroll, hub_x, hub_y, radius, 0.92);
    circle(scroll, hub_x, hub_y, radius * 0.93, 0.65);
    circle(scroll, hub_x, hub_y, radius * 0.73, 0.72);
    let rotation = rotation as f64 * PI / 180.;
    for nth_rune in 0..tongue.repeats {
        let angle = rotation + nth_rune as f64 * 2. * PI / tongue.repeats as f64;
        let degrees = angle * 180. / PI;
        let _ = write!(scroll, "<g transform='translate({hub_x} {hub_y}) rotate({degrees:.3}) scale({radius})' stroke-width='.007'>");
        inscribe_ward_runes(scroll, tongue);
        scroll.push_str("</g>");
    }
    scroll.push_str("</g>");
}
fn unfold_ward_ring(scroll: &mut String, spell: &str) -> Result<(), String> {
    let mut words = spell.split_ascii_whitespace();
    let (hub_x, hub_y, radius) = (
        divine_number(words.next())?,
        divine_number(words.next())?,
        divine_number(words.next())?,
    );
    let rotation = divine_index(words.next())? as u64 % 360;
    let tongue_index = words
        .next()
        .and_then(divine_tongue)
        .ok_or("Unknown ward-ring tongue")?;
    raise_ward_ring(
        scroll,
        hub_x,
        hub_y,
        radius,
        rotation,
        &language_profiles()[tongue_index],
    );
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
fn bind_glyph(scroll: &mut String, glyph: &Outline, width: f64, height: f64, sway: f64, lean: f64) {
    let angle = lean * PI / 180.;
    let (cos, sin) = (angle.cos().abs(), angle.sin().abs());
    let rotated_width = glyph.width * cos + glyph.height * sin;
    let rotated_height = glyph.height * cos + glyph.width * sin;
    let scale = (width / rotated_width).min(height / rotated_height);
    let actual_width = rotated_width * scale;
    let actual_height = rotated_height * scale;
    let _ = write!(scroll, "<use data-glyph='1' data-family='{}' data-width='{actual_width:.6}' data-height='{actual_height:.6}' data-offset='{sway:.6}' href='#glyph-{}' transform='translate(0 {sway:.6}) rotate({lean:.6}) scale({scale:.10} -{scale:.10}) translate({:.6} {:.6})'/>", glyph.family, glyph.id, -glyph.cx, -glyph.cy);
}
/// One annular inscription band of measured proportional glyph cells.
pub(crate) fn write_band(scroll: &mut String, band: &Band) {
    let (hub_x, hub_y, radius, inner, outer) =
        (band.x, band.y, band.radius, band.inner, band.outer);
    let glyph_atlas = glyphs();
    let middle = radius * (inner + outer) / 2.;
    let band_height = radius * (outer - inner);
    let target_height = band_height * 0.81;
    let total = band.cells.iter().map(|cell| cell.weight).sum::<f64>();
    let _ = write!(scroll, "<g data-inscription-band='2' data-inner='{:.6}' data-outer='{:.6}' data-middle='{middle:.6}' data-style='{}' data-m='{hub_x} {hub_y} {radius} {inner} {outer} {} {}", radius * inner, radius * outer, band.style, band.style, band.rotation);
    for cell in &band.cells {
        let _ = write!(scroll, ",{} {} {}", cell.weight, cell.height, cell.lean);
        for glyph in &cell.glyphs {
            let _ = write!(scroll, " {glyph}");
        }
    }
    let _ = write!(
        scroll,
        "' transform='translate({hub_x} {hub_y}) rotate({})' opacity='.94'>",
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
        let _ = write!(scroll, "<g data-cell='1' data-angle='{degrees:.6}' data-sector='{sector:.10}' transform='rotate({degrees:.6}) translate(0 -{middle:.6})'>");
        for (nth_rune, &glyph) in cell.glyphs.iter().enumerate() {
            let (slot_height, sway) = if cell.glyphs.len() == 2 {
                (
                    height * 0.44,
                    height * if nth_rune == 0 { -0.27 } else { 0.27 },
                )
            } else {
                (height, 0.)
            };
            bind_glyph(
                scroll,
                &glyph_atlas[glyph],
                width,
                slot_height,
                sway,
                cell.lean,
            );
        }
        scroll.push_str("</g>");
        angle += sector;
    }
    scroll.push_str("</g>");
}
/// Cell weights follow from glyph proportions, so the compact band stores only each
/// cell's height (steps of 0.002 above 0.9), lean (fifths of a degree) and glyph ids; at
/// inscription scale both steps move a glyph corner by well under a tenth of a pixel.
fn condense_band(spell: &str) -> Result<String, String> {
    let mut parts = spell.split(',');
    let mut scroll = distill(parts.next().unwrap_or_default(), BAND_DECIMALS);
    for part in parts {
        let mut words = part.split_ascii_whitespace();
        let _weight = divine_number(words.next())?;
        let height = ((divine_number(words.next())? - 0.9) * 500.).round() as i64;
        let lean = (divine_number(words.next())? * 5.).round() as i64;
        let _ = write!(scroll, ",{height} {lean}");
        for glyph in words {
            let _ = write!(scroll, " {}", divine_index(Some(glyph))?);
        }
    }
    Ok(scroll)
}
pub(crate) fn rune_weight(glyph_ids: &[usize], height: f64, style: u64, nth_rune: usize) -> f64 {
    let glyph_atlas = self::glyphs();
    let width = glyph_ids
        .iter()
        .map(|&glyph| glyph_atlas[glyph].width / glyph_atlas[glyph].height)
        .reduce(f64::max)
        .unwrap_or(1.);
    width.clamp(0.24, 1.9) * height * if glyph_ids.len() == 2 { 0.46 } else { 1. }
        + 0.19
        + if style == 2 && nth_rune.is_multiple_of(4) {
            0.40
        } else {
            0.
        }
}
fn unfold_band(scroll: &mut String, spell: &str) -> Result<(), String> {
    let mut parts = spell.split(',');
    let mut words = parts.next().unwrap_or_default().split_ascii_whitespace();
    let (hub_x, hub_y, radius) = (
        divine_number(words.next())?,
        divine_number(words.next())?,
        divine_number(words.next())?,
    );
    let (inner, outer) = (divine_number(words.next())?, divine_number(words.next())?);
    let (style, rotation) = (
        divine_index(words.next())? as u64,
        divine_index(words.next())? as u64,
    );
    let mut cells = Vec::new();
    for (nth_rune, part) in parts.enumerate() {
        let mut words = part.split_ascii_whitespace();
        let height = 0.9 + divine_number(words.next())? / 500.;
        let lean = divine_number(words.next())? / 5.;
        let glyph_ids = words
            .map(|glyph| divine_index(Some(glyph)))
            .collect::<Result<Vec<_>, _>>()?;
        if !(1..=2).contains(&glyph_ids.len())
            || glyph_ids.iter().any(|&glyph| glyph >= self::glyphs().len())
        {
            return Err("Invalid magic inscription cell".into());
        }
        cells.push(Cell {
            weight: rune_weight(&glyph_ids, height, style, nth_rune),
            glyphs: glyph_ids,
            height,
            lean,
        });
    }
    if cells.is_empty() || cells.len() > 256 || !(radius > 0. && inner < outer) {
        return Err("Invalid magic inscription band".into());
    }
    write_band(
        scroll,
        &Band {
            x: hub_x,
            y: hub_y,
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
fn unfold_polygon(scroll: &mut String, spell: &str) -> Result<(), String> {
    let mut words = spell.split_ascii_whitespace();
    let (hub_x, hub_y, radius) = (
        divine_number(words.next())?,
        divine_number(words.next())?,
        divine_number(words.next())?,
    );
    let (sides, skip) = (divine_index(words.next())?, divine_index(words.next())?);
    if !(1..=64).contains(&sides) {
        return Err("Invalid magic polygon".into());
    }
    poly(
        scroll,
        hub_x,
        hub_y,
        radius,
        sides,
        skip,
        divine_number(words.next())?,
    );
    Ok(())
}

/// The exact reversible name contour: one radial vertex per frame nibble.
pub(crate) fn write_sigil(
    scroll: &mut String,
    entity: usize,
    hub_x: f64,
    hub_y: f64,
    seal_radius: f64,
    nibbles: &[u8],
) {
    let length = nibbles.len();
    let core = seal_radius * 0.52;
    let _ = write!(
        scroll,
        "<polygon data-name-sigil='1' data-entity='{entity}' data-seal-radius='{seal_radius}' data-m='{entity} {hub_x} {hub_y} {seal_radius} "
    );
    for &nibble in nibbles {
        scroll.push(char::from_digit(nibble as u32, 16).unwrap());
    }
    scroll.push_str("' points='");
    for (nth_rune, &nibble) in nibbles.iter().enumerate() {
        let angle = -PI / 2. + nth_rune as f64 * 2. * PI / length as f64;
        let reach = 0.72 + nibble as f64 * 0.014;
        let _ = write!(
            scroll,
            "{}{:.6},{:.6}",
            if nth_rune == 0 { "" } else { " " },
            (reach * angle.cos() * 1_000_000.).round() / 1_000_000.,
            (reach * angle.sin() * 1_000_000.).round() / 1_000_000.
        );
    }
    let _ = write!(scroll, "' transform='translate({hub_x} {hub_y}) scale({core})' stroke-width='.007' opacity='.65' stroke-linejoin='round'/>");
}
pub(crate) fn sigil_nibbles(points: &[[f64; 2]]) -> Vec<u8> {
    points
        .iter()
        .map(|vertex| {
            ((vertex[0].hypot(vertex[1]) - 0.72) / 0.014)
                .round()
                .clamp(0., 15.) as u8
        })
        .collect()
}
fn unfold_sigil(scroll: &mut String, spell: &str) -> Result<(), String> {
    let mut words = spell.split_ascii_whitespace();
    let entity = divine_index(words.next())?;
    let (hub_x, hub_y, radius) = (
        divine_number(words.next())?,
        divine_number(words.next())?,
        divine_number(words.next())?,
    );
    let nibbles = words
        .next()
        .ok_or("Truncated magic macro")?
        .chars()
        .map(|mark| mark.to_digit(16).map(|digit| digit as u8))
        .collect::<Option<Vec<_>>>()
        .ok_or("Invalid name contour")?;
    write_sigil(scroll, entity, hub_x, hub_y, radius, &nibbles);
    Ok(())
}

/// Round decimal tokens only. Colors, indices and nibble strings are never numbers here.
fn distill(spell: &str, precision: usize) -> String {
    let mut scroll = String::with_capacity(spell.len());
    let mut word = String::new();
    let release = |word: &mut String, scroll: &mut String| {
        match word.parse::<f64>() {
            Ok(value) if word.contains('.') && value.is_finite() => {
                let mut inscription = format!("{value:.precision$}");
                while inscription.contains('.') && inscription.ends_with('0') {
                    inscription.pop();
                }
                if inscription.ends_with('.') {
                    inscription.pop();
                }
                scroll.push_str(if inscription == "-0" {
                    "0"
                } else {
                    &inscription
                });
            }
            _ => scroll.push_str(word),
        }
        word.clear();
    };
    for mark in spell.chars() {
        if mark == ' ' || mark == ',' {
            release(&mut word, &mut scroll);
            scroll.push(mark);
        } else {
            word.push(mark);
        }
    }
    release(&mut word, &mut scroll);
    scroll
}
fn macro_sigil(rune_node: &roxmltree::Node<'_, '_>) -> Option<(&'static str, usize)> {
    rune_node.attribute("data-m")?;
    Some(if rune_node.has_attribute("data-channel") {
        ("B", BRAID_DECIMALS)
    } else if rune_node.has_attribute("data-ward-ring") {
        ("W", WARD_DECIMALS)
    } else if rune_node.has_attribute("data-inscription-band") {
        ("I", BAND_DECIMALS)
    } else if rune_node.has_attribute("data-name-sigil") {
        ("N", SIGIL_DECIMALS)
    } else if rune_node.has_tag_name("polyline") {
        ("Y", POLYGON_DECIMALS)
    } else {
        return None;
    })
}

fn etch_number(scroll: &mut String, value: f64) {
    let value = if value == 0. { 0. } else { value };
    let inscription = format!("{value}");
    let glued =
        inscription.starts_with('-') || scroll.ends_with(|mark: char| mark.is_ascii_alphabetic());
    if !glued && !scroll.is_empty() {
        scroll.push(' ');
    }
    scroll.push_str(&inscription);
}

/// Rewrite a glyph outline with relative commands on a grid of `GLYPH_GRID` font units. The
/// grid is tracked in absolute rounded coordinates, so rounding never accumulates;
/// at inscription scale one font unit is roughly a sixtieth of a pixel, so the 2-unit grid
/// moves an outline point by at most about a sixtieth of a pixel.
fn relative_trail(trail: &str, lattice: f64) -> Result<String, String> {
    let ink_bytes = trail.as_bytes();
    let mut runic_tokens: Vec<Result<char, f64>> = Vec::new();
    let mut cursor = 0;
    while cursor < ink_bytes.len() {
        let mark = ink_bytes[cursor] as char;
        if mark.is_ascii_alphabetic() {
            runic_tokens.push(Ok(mark));
            cursor += 1;
        } else if mark == '-' || mark == '+' || mark == '.' || mark.is_ascii_digit() {
            let opening = cursor;
            cursor += 1;
            let mut seen_dot = mark == '.';
            while cursor < ink_bytes.len() {
                let next_mark = ink_bytes[cursor] as char;
                if next_mark.is_ascii_digit() || (next_mark == '.' && !seen_dot) {
                    seen_dot |= next_mark == '.';
                    cursor += 1;
                } else if (next_mark == 'e' || next_mark == 'E') && cursor + 1 < ink_bytes.len() {
                    cursor += 1;
                    if matches!(ink_bytes[cursor], b'-' | b'+') {
                        cursor += 1;
                    }
                } else {
                    break;
                }
            }
            runic_tokens.push(Err(trail[opening..cursor]
                .parse()
                .map_err(|_| "Invalid glyph path")?));
        } else {
            cursor += 1;
        }
    }
    let mut scroll = String::new();
    let (mut true_x, mut true_y, mut round_x, mut round_y) = (0., 0., 0., 0.);
    let (mut start_true, mut start_round) = ([0., 0.], [0., 0.]);
    let mut words = runic_tokens.into_iter().peekable();
    let mut command = 'M';
    while words.peek().is_some() {
        if let Some(Ok(letter)) = words.peek() {
            command = *letter;
            words.next();
            if matches!(command, 'Z' | 'z') {
                scroll.push('z');
                (true_x, true_y) = (start_true[0], start_true[1]);
                (round_x, round_y) = (start_round[0], start_round[1]);
                continue;
            }
            scroll.push(command.to_ascii_lowercase());
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
        let mut next_word = || match words.next() {
            Some(Err(value)) => Ok(value),
            _ => Err("Invalid glyph path".to_string()),
        };
        let plot = |next_word: &mut dyn FnMut() -> Result<f64, String>,
                    scroll: &mut String,
                    anchor: (f64, f64, f64, f64)|
         -> Result<(f64, f64), String> {
            let (east, north) = (next_word()?, next_word()?);
            let (abs_east, abs_north) = if relative {
                (anchor.0 + east, anchor.1 + north)
            } else {
                (east, north)
            };
            etch_number(scroll, (abs_east / lattice).round() - anchor.2);
            etch_number(scroll, (abs_north / lattice).round() - anchor.3);
            Ok((abs_east, abs_north))
        };
        let anchor = (true_x, true_y, round_x, round_y);
        let landing = match upper {
            'M' | 'L' => plot(&mut next_word, &mut scroll, anchor)?,
            'C' => {
                plot(&mut next_word, &mut scroll, anchor)?;
                plot(&mut next_word, &mut scroll, anchor)?;
                plot(&mut next_word, &mut scroll, anchor)?
            }
            'Q' => {
                plot(&mut next_word, &mut scroll, anchor)?;
                plot(&mut next_word, &mut scroll, anchor)?
            }
            'H' => {
                let east = next_word()?;
                let abs_east = if relative { true_x + east } else { east };
                etch_number(&mut scroll, (abs_east / lattice).round() - round_x);
                (abs_east, true_y)
            }
            'V' => {
                let north = next_word()?;
                let abs_north = if relative { true_y + north } else { north };
                etch_number(&mut scroll, (abs_north / lattice).round() - round_y);
                (true_x, abs_north)
            }
            'A' => {
                for _ in 0..2 {
                    let radius = next_word()?;
                    etch_number(&mut scroll, (radius / lattice).round().max(1.));
                }
                for _ in 0..3 {
                    let value = next_word()?;
                    etch_number(&mut scroll, value);
                }
                plot(&mut next_word, &mut scroll, anchor)?
            }
            _ => return Err("Unsupported glyph path command".into()),
        };
        (true_x, true_y) = landing;
        (round_x, round_y) = ((landing.0 / lattice).round(), (landing.1 / lattice).round());
        if upper == 'M' {
            start_true = [true_x, true_y];
            start_round = [round_x, round_y];
        }
    }
    Ok(scroll)
}

/// Replace every parameterized subtree by its macro and add what the embedded
/// expander needs: glyph metrics, ward-rune templates and the script itself.
pub(crate) fn compact(vellum: &str) -> Result<String, String> {
    if guard_format(vellum)? {
        return Ok(vellum.into());
    }
    let tome = roxmltree::Document::parse(vellum).map_err(|_| "Invalid SVG artwork")?;
    let taproot = tome.root_element();
    if !taproot.has_tag_name("svg") || vellum.contains("<script") {
        return Err("Invalid SVG artwork".into());
    }
    let glyph_atlas = glyphs();
    let mut transmutations: Vec<(std::ops::Range<usize>, String)> = Vec::new();
    let mut warded_tongues = std::collections::BTreeSet::new();
    let mut veiled_until = 0;
    for rune_node in tome
        .descendants()
        .filter(|rune_node| rune_node.is_element())
    {
        if rune_node.range().start < veiled_until {
            continue;
        }
        if let Some((incantation, precision)) = macro_sigil(&rune_node) {
            let spell = rune_node.attribute("data-m").unwrap();
            if incantation == "W" {
                if let Some(tongue_id) = spell.split_ascii_whitespace().nth(4) {
                    warded_tongues
                        .insert(divine_tongue(tongue_id).ok_or("Unknown ward-ring tongue")?);
                }
            }
            let spell = if incantation == "I" {
                condense_band(spell)?
            } else {
                distill(spell, precision)
            };
            veiled_until = rune_node.range().end;
            transmutations.push((rune_node.range(), format!("<{incantation} d='{spell}'/>")));
        } else if rune_node.has_tag_name("path")
            && rune_node
                .parent_element()
                .is_some_and(|parent| parent.has_tag_name("defs"))
        {
            let Some(glyph_id) = rune_node
                .attribute("id")
                .and_then(|id| id.strip_prefix("glyph-")?.parse::<usize>().ok())
            else {
                continue;
            };
            let gridded = rune_node.attribute("transform") == Some(GLYPH_SCALE);
            let trail = rune_node.attribute("d").ok_or("Invalid glyph definition")?;
            let trail = relative_trail(trail, if gridded { 1. } else { GLYPH_GRID })?;
            let metrics = glyph_atlas.get(glyph_id).map_or(String::new(), |glyph| {
                format!(
                    " k='{} {} {} {}'",
                    glyph.cx, glyph.cy, glyph.width, glyph.height
                )
            });
            transmutations.push((
                rune_node.range(),
                format!(
                    "<path id='glyph-{glyph_id}' transform='{GLYPH_SCALE}' d='{trail}'{metrics}/>"
                ),
            ));
        }
    }
    let mut scroll = vellum.to_string();
    for (span, inscription) in transmutations.into_iter().rev() {
        scroll.replace_range(span, &inscription);
    }
    let mut ward_runes = String::new();
    for &tongue_index in &warded_tongues {
        let tongue = &language_profiles()[tongue_index];
        let _ = write!(
            ward_runes,
            "<g id='W{tongue_index}' l='{}' n='{}' stroke-width='.007'>",
            tongue.id, tongue.repeats
        );
        inscribe_ward_runes(&mut ward_runes, tongue);
        ward_runes.push_str("</g>");
    }
    match scroll.find("</defs>") {
        Some(seam) => scroll.insert_str(seam, &ward_runes),
        None => {
            let seam = scroll.find('>').ok_or("Invalid SVG artwork")? + 1;
            scroll.insert_str(seam, &format!("<defs>{ward_runes}</defs>"));
        }
    }
    let finale = scroll.rfind("</svg>").ok_or("Invalid SVG artwork")?;
    scroll.insert_str(finale, &format!("<script><![CDATA[{SCRIPT}]]></script>"));
    let prologue = scroll.find("<svg ").ok_or("Invalid SVG artwork")? + 5;
    scroll.insert_str(prologue, &format!("{FORMAT} "));
    Ok(scroll)
}

/// Expand a compact magic SVG back to the renderer's static markup. Other SVG passes
/// through unchanged.
pub fn expand_svg(vellum: &str) -> Result<String, String> {
    if !guard_format(vellum)? {
        return Ok(vellum.into());
    }
    let tome = roxmltree::Document::parse(vellum).map_err(|_| "Invalid magic SVG")?;
    let taproot = tome.root_element();
    if !taproot.has_tag_name("svg") {
        return Err("Invalid magic SVG".into());
    }
    let mut transmutations: Vec<(std::ops::Range<usize>, String)> = Vec::new();
    for rune_node in tome
        .descendants()
        .filter(|rune_node| rune_node.is_element())
    {
        let incantation = rune_node.tag_name().name();
        let mut unfurled = String::new();
        match incantation {
            "B" | "W" | "I" | "N" | "Y" => {
                let spell = rune_node.attribute("d").ok_or("Empty magic macro")?;
                match incantation {
                    "B" => write_braid(&mut unfurled, &Braid::parse(spell)?),
                    "W" => unfold_ward_ring(&mut unfurled, spell)?,
                    "I" => unfold_band(&mut unfurled, spell)?,
                    "Y" => unfold_polygon(&mut unfurled, spell)?,
                    _ => unfold_sigil(&mut unfurled, spell)?,
                }
                transmutations.push((rune_node.range(), unfurled));
            }
            "script" => transmutations.push((rune_node.range(), unfurled)),
            "g" if rune_node
                .attribute("id")
                .is_some_and(|ward_id| ward_id.starts_with('W'))
                && rune_node
                    .parent_element()
                    .is_some_and(|parent| parent.has_tag_name("defs")) =>
            {
                transmutations.push((rune_node.range(), unfurled))
            }
            "path" => {
                if let Some(metrics) = rune_node.attribute_node("k") {
                    let span = metrics.range();
                    transmutations.push((span.start - 1..span.end, unfurled));
                }
            }
            _ => {}
        }
    }
    transmutations.sort_by_key(|(span, _)| span.start);
    let mut scroll = vellum.to_string();
    let mut veiled_from = usize::MAX;
    for (span, inscription) in transmutations.into_iter().rev() {
        if span.end > veiled_from {
            return Err("Overlapping magic macros".into());
        }
        veiled_from = span.start;
        scroll.replace_range(span, &inscription);
    }
    Ok(scroll.replacen(&format!("{FORMAT} "), "", 1))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{analyze, render_with_key, MagicKey, SourceFile};

    fn drawing(seal_key: Option<&MagicKey>) -> String {
        let mut files = Vec::new();
        for nth_rune in 0..24 {
            let body = "    let x = 1;\n".repeat(1 + nth_rune * 3);
            let next = (nth_rune + 1) % 24;
            files.push(SourceFile {
                path: format!("m{nth_rune}.{}", ["rs", "py"][nth_rune % 2]),
                content: if nth_rune % 2 == 0 {
                    format!(
                        "fn s{nth_rune}() {{\n{body}    s{next}();\n    s{}();\n}}\n",
                        (nth_rune + 2) % 24
                    )
                } else {
                    format!(
                        "def s{nth_rune}():\n{}    return s{}()\n",
                        body.replace("let ", ""),
                        (nth_rune + 2) % 24
                    )
                },
            });
        }
        let analysis = analyze(&files);
        render_with_key(&analysis, "t", 4, "gold", false, true, seal_key)
            .unwrap()
            .svg
    }

    #[test]
    fn unquantized_macros_reproduce_renderer_markup_exactly() {
        let vellum = drawing(None);
        let tome = roxmltree::Document::parse(&vellum).unwrap();
        let mut checked = BTreeMap::new();
        for rune_node in tome
            .descendants()
            .filter(|rune_node| rune_node.is_element())
        {
            let Some((incantation, _)) = macro_sigil(&rune_node) else {
                continue;
            };
            let spell = rune_node.attribute("data-m").unwrap();
            let mut unfurled = String::new();
            match incantation {
                "B" => write_braid(&mut unfurled, &Braid::parse(spell).unwrap()),
                "W" => unfold_ward_ring(&mut unfurled, spell).unwrap(),
                "N" => unfold_sigil(&mut unfurled, spell).unwrap(),
                "Y" => unfold_polygon(&mut unfurled, spell).unwrap(),
                _ => continue,
            }
            assert_eq!(
                unfurled,
                &vellum[rune_node.range()],
                "{incantation} macro diverged"
            );
            *checked.entry(incantation).or_insert(0) += 1;
        }
        assert_eq!(checked.len(), 4, "{checked:?}");
    }

    #[test]
    fn compact_artwork_is_small_scripted_idempotent_and_name_exact() {
        for seal_key in [None, Some(MagicKey::from_hex(&"07".repeat(32)).unwrap())] {
            let full = drawing(seal_key.as_ref());
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
                crate::decode_svg_names_with_key(&compact, seal_key.as_ref()).unwrap(),
                crate::decode_svg_names_with_key(&full, seal_key.as_ref()).unwrap()
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
        let relative = relative_trail(absolute, 1.).unwrap();
        assert_eq!(&relative[..21], "m-23-410l0 820h46v-82");
        // Relative input describes the same outline and is a fixed point.
        assert_eq!(relative_trail(&relative, 1.).unwrap(), relative);
        assert_eq!(relative_trail("m1 1 2 2l3 3", 1.).unwrap(), "m1 1 2 2l3 3");
        assert_eq!(relative_trail("M10 10 20 20", 2.).unwrap(), "m5 5 5 5");
        assert!(relative_trail("M1", 1.).is_err());
    }

    #[test]
    fn older_macro_dialects_are_rejected_not_half_unfolded() {
        let format_three = "<svg data-magic-format='3' xmlns='http://www.w3.org/2000/svg'><O d='10 10 40 0 rust'/></svg>";
        for verdict in [expand_svg(format_three), compact(format_three)] {
            assert!(verdict.unwrap_err().contains("Unsupported magic format"));
        }
        assert!(crate::decode_svg_names(format_three).is_err());
    }

    #[test]
    fn macro_quantization_never_touches_colors_or_nibbles() {
        assert_eq!(
            distill("#1e5a3b 1e5 0.123456,-0.00001 c71e5", 2),
            "#1e5a3b 1e5 0.12,0 c71e5"
        );
        assert!(expand_svg("<svg data-magic-format='3' xmlns='http://www.w3.org/2000/svg'><B d='#zz0000 #000000 calls 0 1 3 4 0 0 1 1 2 2 3 3'/></svg>").is_err());
        assert!(expand_svg("<svg data-magic-format='3' xmlns='http://www.w3.org/2000/svg'><I d='0 0 10 0.7 0.9 0 0,1 1 99999'/></svg>").is_err());
    }
}
