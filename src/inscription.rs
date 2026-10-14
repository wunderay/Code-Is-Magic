//! Measured inscriptions: proportional glyphs packed into annular cells.
//! Decorative sampling is separate from the lossless name/source codecs.
use crate::render::{entity_seed, hash, Entropy};
use crate::{Entity, MagicKey};
use serde::Deserialize;
use std::{collections::BTreeMap, f64::consts::PI, fmt::Write};

#[derive(Deserialize)]
pub(crate) struct Outline {
    #[serde(skip)]
    pub id: usize,
    pub path: String,
    pub cx: f64,
    pub cy: f64,
    pub width: f64,
    pub height: f64,
    pub family: String,
}

pub(crate) struct Atlas {
    pub outlines: BTreeMap<String, Outline>,
    alphabet: BTreeMap<String, String>,
    families: BTreeMap<String, Vec<String>>,
}
impl Atlas {
    pub fn new(key: Option<&MagicKey>) -> Self {
        let mut outlines: BTreeMap<String, Outline> =
            serde_json::from_str(include_str!("glyphs.json")).expect("bundled measured glyphs");
        for (id, outline) in outlines.values_mut().enumerate() {
            outline.id = id;
        }
        let symbols: Vec<_> = outlines.keys().cloned().collect();
        let mut shuffled = symbols.clone();
        if let Some(key) = key {
            shuffled.sort_by_key(|symbol| {
                (
                    key.seed(b"glyph-alphabet", symbol.as_bytes()),
                    symbol.clone(),
                )
            });
        }
        let alphabet = symbols.into_iter().zip(shuffled).collect();
        let mut families: BTreeMap<String, Vec<String>> = BTreeMap::new();
        for (symbol, outline) in &outlines {
            families
                .entry(outline.family.clone())
                .or_default()
                .push(symbol.clone());
        }
        Self {
            outlines,
            alphabet,
            families,
        }
    }
    pub fn definitions(&self, svg: &str) -> String {
        let used: std::collections::BTreeSet<usize> = svg
            .split("href='#glyph-")
            .skip(1)
            .filter_map(|s| s.split_once('\'')?.0.parse().ok())
            .collect();
        let mut defs = String::from("<defs>");
        for glyph in self.outlines.values().filter(|g| used.contains(&g.id)) {
            let _ = write!(defs, "<path id='glyph-{}' d='{}'/>", glyph.id, glyph.path);
        }
        defs.push_str("</defs>");
        defs
    }
    fn choose(&self, family: &str, seed: u64) -> &Outline {
        let pool = &self.families[family];
        self.symbol(&pool[(seed % pool.len() as u64) as usize])
    }
    fn symbol(&self, symbol: &str) -> &Outline {
        &self.outlines[&self.alphabet[symbol]]
    }
}
const FAMILIES: [&str; 10] = [
    "runic",
    "ogham",
    "greek",
    "alchemical",
    "celestial",
    "geometric",
    "dingbat",
    "mathematical",
    "ancient-marks",
    "bind-sigils",
];
fn representative(token: &str) -> Option<&'static str> {
    Some(match token {
        "if" | "else" | "match" | "switch" => "Δ",
        "for" | "while" | "loop" => "∞",
        "return" | "yield" => "Ω",
        "let" | "const" | "var" => "ᚠ",
        "=" | "=>" => "ᚷ",
        "+" | "+=" => "✚",
        "-" => "ᚾ",
        "*" => "✶",
        "/" => "ϕ",
        "(" | ")" => "☾",
        "{" | "}" => "⊹",
        "[" | "]" => "◇",
        ":" | "::" | "." => "☿",
        ";" | "," => "∴",
        "==" | "!=" | "<" | ">" | "<=" | ">=" => "⚖",
        "&&" | "and" => "ᛗ",
        "||" | "or" => "ᛞ",
        "async" | "await" => "☽",
        "try" | "catch" | "?" => "Ψ",
        "literal" => "◈",
        "self" | "this" => "Θ",
        "pub" | "public" => "☉",
        "mut" => "ᛏ",
        "new" => "✧",
        "true" | "false" | "None" | "null" => "☯",
        _ => return None,
    })
}
struct Cell<'a> {
    glyphs: Vec<&'a Outline>,
    height: f64,
    lean: f64,
    weight: f64,
}
#[allow(clippy::too_many_arguments)]
fn cells<'a>(
    atlas: &'a Atlas,
    entity: &Entity,
    seed: u64,
    row: usize,
    count: usize,
    style: u64,
    wingdings: bool,
) -> Vec<Cell<'a>> {
    let primary = (seed % 10) as usize;
    let secondary = (primary + 1 + ((seed >> 13) % 9) as usize) % 10;
    (0..count)
        .map(|i| {
            let token = entity
                .tokens
                .get(i * entity.tokens.len().max(1) / count)
                .map_or(entity.name.as_str(), |s| s.as_str());
            let mut ink = Entropy(
                seed ^ hash(token) ^ (i as u64 + row as u64 * 131).wrapping_mul(0x9e3779b97f4a7c15),
            );
            let mixed = ink.0;
            let family = if wingdings && i % 3 == 0 {
                "dingbat"
            } else if i % 7 == 0 {
                FAMILIES[(mixed % 10) as usize]
            } else {
                FAMILIES[if (i + row).is_multiple_of(3) {
                    secondary
                } else {
                    primary
                }]
            };
            let glyph = if mixed % 5 < 2 && !(wingdings && i % 3 == 0) {
                representative(token).map_or_else(
                    || atlas.choose(family, mixed >> 7),
                    |symbol| atlas.symbol(symbol),
                )
            } else {
                atlas.choose(family, mixed >> 7)
            };
            let mut glyphs = vec![glyph];
            if style == 3 && i % 4 == 0 {
                glyphs.push(atlas.choose(FAMILIES[secondary], mixed.wrapping_add(71)));
            }
            let height = ink.between(0.90, 1.04);
            let width = glyphs
                .iter()
                .map(|g| g.width / g.height)
                .reduce(f64::max)
                .unwrap();
            let weight =
                width.clamp(0.24, 1.9) * height * if glyphs.len() == 2 { 0.46 } else { 1. }
                    + 0.19
                    + if style == 2 && i % 4 == 0 { 0.40 } else { 0. };
            Cell {
                glyphs,
                height,
                lean: ink.between(-4., 4.),
                weight,
            }
        })
        .collect()
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

#[allow(clippy::too_many_arguments)]
pub(crate) fn inscribe(
    s: &mut String,
    atlas: &Atlas,
    entity: &Entity,
    x: f64,
    y: f64,
    radius: f64,
    large: bool,
    wingdings: bool,
    key: Option<&MagicKey>,
) {
    let seed = entity_seed(entity, key).wrapping_add(17);
    let style = seed % 4;
    let bands: &[(f64, f64)] = if style == 1 && large && radius >= 180. {
        &[(0.744, 0.823), (0.844, 0.925)]
    } else {
        &[(0.744, 0.925)]
    };
    for (row, &(inner, outer)) in bands.iter().enumerate() {
        let middle = radius * (inner + outer) / 2.;
        let band_height = radius * (outer - inner);
        let target_height = band_height * 0.81;
        // Estimate density from actual glyph proportions, then pack the final cells.
        let sample = cells(atlas, entity, seed, row, 32, style, wingdings);
        let mean = sample.iter().map(|c| c.weight).sum::<f64>() / sample.len() as f64;
        let count = ((2. * PI * radius * inner) / (target_height * mean))
            .round()
            .clamp(12., 88.) as usize;
        let cells = cells(atlas, entity, seed, row, count, style, wingdings);
        let total = cells.iter().map(|c| c.weight).sum::<f64>();
        let rotation = ((seed >> (row * 7)) % 360) as f64;
        let _ = write!(s, "<g data-inscription-band='2' data-inner='{:.6}' data-outer='{:.6}' data-middle='{middle:.6}' data-style='{style}' transform='translate({x} {y}) rotate({rotation})' opacity='.94'>", radius * inner, radius * outer);
        let mut angle = 0.;
        for cell in cells {
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
            for (i, glyph) in cell.glyphs.iter().enumerate() {
                let (h, offset) = if cell.glyphs.len() == 2 {
                    (height * 0.44, height * if i == 0 { -0.27 } else { 0.27 })
                } else {
                    (height, 0.)
                };
                fit_glyph(s, glyph, width, h, offset, cell.lean);
            }
            s.push_str("</g>");
            angle += sector;
        }
        s.push_str("</g>");
    }
}
