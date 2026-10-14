use crate::encode_name;
use crate::{Analysis, Entity};
use serde::Deserialize;
use serde::Serialize;
use std::collections::BTreeMap;
use std::f64::consts::PI;
use std::fmt::Write;

#[derive(Serialize)]
pub struct Placement {
    pub id: usize,
    pub x: f64,
    pub y: f64,
    pub radius: f64,
}
#[derive(Serialize)]
pub struct Diagram {
    pub svg: String,
    pub width: u32,
    pub height: u32,
    pub placements: Vec<Placement>,
    pub analysis: Analysis,
}
fn hash(s: &str) -> u64 {
    s.bytes().fold(14695981039346656037, |h, b| {
        (h ^ b as u64).wrapping_mul(1099511628211)
    })
}
// Source-seeded entropy keeps the chaos identical in native Rust and WebAssembly.
struct Entropy(u64);
impl Entropy {
    fn unit(&mut self) -> f64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (self.0 >> 11) as f64 / (1_u64 << 53) as f64
    }
    fn between(&mut self, low: f64, high: f64) -> f64 {
        low + self.unit() * (high - low)
    }
}
fn entity_seed(e: &Entity) -> u64 {
    hash(&e.qualified_name)
}
fn scatter_layout(a: &Analysis, visible: &[&Entity]) -> (Vec<Placement>, u32, u32) {
    let mut neighbors = vec![Vec::new(); a.entities.len()];
    for edge in &a.edges {
        if edge.from != edge.to {
            neighbors[edge.from].push(edge.to);
            neighbors[edge.to].push(edge.from);
        }
    }
    // Function area is proportional to non-comment body source lines.
    // Name-derived variation belongs to the contour, never to function size.
    let radius = |e: &Entity| {
        if e.kind == "function" {
            24. * (e.lines.max(1) as f64).sqrt()
        } else {
            92. + (entity_seed(e) % 37) as f64
        }
    };
    // A neighboring-cell lookup must remain safe even for very large functions.
    let max_radius = visible
        .iter()
        .map(|e| radius(e))
        .reduce(f64::max)
        .unwrap_or(24.);
    let cell_size = (2. * max_radius + 82.).max(128.);
    let spread = visible
        .iter()
        .map(|e| (radius(e) + 50.).powi(2))
        .sum::<f64>()
        .sqrt()
        .max(180.);
    let mut order = visible.to_vec();
    order.sort_by(|left, right| {
        neighbors[right.id]
            .len()
            .cmp(&neighbors[left.id].len())
            .then_with(|| radius(right).total_cmp(&radius(left)))
            .then_with(|| left.id.cmp(&right.id))
    });
    let mut placements: Vec<Placement> = Vec::new();
    let mut placed = vec![None; a.entities.len()];
    let mut cells: BTreeMap<(i32, i32), Vec<usize>> = BTreeMap::new();
    for e in order {
        let r = radius(e);
        let mut entropy = Entropy(entity_seed(e));
        let anchors: Vec<usize> = neighbors[e.id]
            .iter()
            .filter_map(|&id| placed[id])
            .collect();
        let (mut x, mut y, mut attempt) = (0., 0., 0_usize);
        loop {
            if !placements.is_empty() {
                let angle = entropy.between(0., 2. * PI);
                if !anchors.is_empty() && attempt < 64 {
                    let anchor = &placements[anchors[attempt % anchors.len()]];
                    let distance = r + anchor.radius + entropy.between(85., 260.);
                    x = anchor.x + angle.cos() * distance;
                    y = anchor.y + angle.sin() * distance;
                } else {
                    let distance = spread * entropy.unit().sqrt() * (0.9 + attempt as f64 / 240.);
                    x = distance * angle.cos() * 1.09;
                    y = distance * angle.sin() * 0.93;
                }
            }
            let cell = (
                (x / cell_size).floor() as i32,
                (y / cell_size).floor() as i32,
            );
            let collision = (-1..=1).any(|dx| {
                (-1..=1).any(|dy| {
                    cells
                        .get(&(cell.0 + dx, cell.1 + dy))
                        .is_some_and(|bucket| {
                            bucket.iter().any(|&index| {
                                let p = &placements[index];
                                (p.x - x).hypot(p.y - y) < p.radius + r + 82.
                            })
                        })
                })
            });
            if !collision {
                cells.entry(cell).or_default().push(placements.len());
                placed[e.id] = Some(placements.len());
                placements.push(Placement {
                    id: e.id,
                    x,
                    y,
                    radius: r,
                });
                break;
            }
            attempt += 1;
        }
    }
    let left = placements
        .iter()
        .map(|p| p.x - p.radius * 1.24)
        .reduce(f64::min)
        .unwrap_or(0.);
    let top = placements
        .iter()
        .map(|p| p.y - p.radius * 1.24)
        .reduce(f64::min)
        .unwrap_or(0.);
    let right = placements
        .iter()
        .map(|p| p.x + p.radius * 1.24)
        .reduce(f64::max)
        .unwrap_or(760.);
    let bottom = placements
        .iter()
        .map(|p| p.y + p.radius * 1.24 + 50.)
        .reduce(f64::max)
        .unwrap_or(440.);
    for p in &mut placements {
        p.x = ((p.x - left + 75.) * 1000.).round() / 1000.;
        p.y = ((p.y - top + 150.) * 1000.).round() / 1000.;
    }
    placements.sort_by_key(|p| p.id);
    (
        placements,
        (right - left + 150.).ceil().max(960.) as u32,
        (bottom - top + 235.).ceil().max(720.) as u32,
    )
}
fn arc(s: &mut String, x: f64, y: f64, r: f64, start: f64, sweep: f64, opacity: f64) {
    let end = start + sweep;
    let _ = write!(
        s,
        "<path d='M {:.4} {:.4} A {r:.4} {r:.4} 0 {} 1 {:.4} {:.4}' opacity='{opacity}'/>",
        x + r * start.cos(),
        y + r * start.sin(),
        usize::from(sweep > PI),
        x + r * end.cos(),
        y + r * end.sin()
    );
}
fn polygon(cx: f64, cy: f64, r: f64, n: usize, step: usize, rotation: f64) -> String {
    (0..=n)
        .map(|i| {
            let a = rotation + (i * step % n) as f64 * 2.0 * PI / n as f64;
            format!("{:.4},{:.4}", cx + r * a.cos(), cy + r * a.sin())
        })
        .collect::<Vec<_>>()
        .join(" ")
}
fn glyph(token: &str) -> &'static str {
    match token {
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
        ";" | "," => "·",
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
        _ => {
            const RUNES: [&str; 36] = [
                "ᚠ", "ᚢ", "ᚦ", "ᚨ", "ᚱ", "ᚲ", "ᚷ", "ᚹ", "ᚺ", "ᚾ", "ᛁ", "ᛃ", "ᛇ", "ᛈ", "ᛉ", "ᛊ",
                "ᛏ", "ᛒ", "ᛖ", "ᛗ", "ᛚ", "ᛜ", "ᛞ", "ᛟ", "α", "β", "γ", "λ", "ξ", "π", "ϙ", "ϝ",
                "ϟ", "ϡ", "∴", "∵",
            ];
            RUNES[(hash(token) % RUNES.len() as u64) as usize]
        }
    }
}
fn circle(s: &mut String, x: f64, y: f64, r: f64, opacity: f64) {
    let _ = write!(
        s,
        "<circle cx='{x:.4}' cy='{y:.4}' r='{r:.4}' opacity='{opacity}'/>"
    );
}
fn poly(s: &mut String, x: f64, y: f64, r: f64, n: usize, step: usize, rot: f64) {
    let _ = write!(s, "<polyline points='{}'/>", polygon(x, y, r, n, step, rot));
}
// Original emblems inspired by ritual diagrams, drawn in normalized local coordinates.
// The exact name contour remains a separate lossless geometry layer.
fn center_emblem(s: &mut String, e: &Entity, x: f64, y: f64, r: f64) {
    let seed = entity_seed(e);
    let mut ink = Entropy(seed.wrapping_add(913));
    let family = (seed ^ (seed >> 23)) % 12;
    let rotation = ink.between(-180., 180.);
    let span = r * ink.between(0.58, 0.66);
    let weight = 0.014_f64.max(0.9 / span);
    let _ = write!(s, "<g data-center='{family}' transform='translate({x} {y}) rotate({rotation:.3}) scale({span:.3})' stroke-width='{weight:.6}' stroke-linejoin='round' stroke-linecap='round'>");
    if e.kind != "function" {
        match e.kind.as_str() {
            "class" | "impl" => {
                poly(s, 0., 0., 0.94, 4, 1, PI / 4.);
                poly(s, 0., 0., 0.74, 4, 1, PI / 4.);
                s.push_str("<path d='M -.66 -.4 L .66 -.4 M -.66 .4 L .66 .4 M -.4 -.66 L -.4 .66 M .4 -.66 L .4 .66'/>");
            }
            "enum" => {
                poly(s, 0., 0., 1., 4, 1, 0.);
                poly(s, 0., 0., 0.67, 4, 1, 0.);
                s.push_str("<path d='M 0 -1 L 0 -.67 M 1 0 L .67 0 M 0 1 L 0 .67 M -1 0 L -.67 0'/>");
            }
            "interface" => {
                poly(s, 0., 0., 1., 3, 1, -PI / 2.);
                poly(s, 0., 0., 0.94, 3, 1, PI / 2.);
            }
            "object" => s.push_str("<path d='M 0 -.95 L .82 -.48 L .82 .48 L 0 .95 L -.82 .48 L -.82 -.48 Z M 0 0 L 0 .95 M 0 0 L .82 -.48 M 0 0 L -.82 -.48'/>"),
            _ => {
                poly(s, 0., 0., 1., 6, 1, PI / 6.);
                poly(s, 0., 0., 0.73, 6, 1, PI / 6.);
            }
        }
    } else {
        match family {
            0 => {
                poly(s, 0., 0., 1., 3, 1, -PI / 2.);
                poly(s, 0., 0., ink.between(0.77, 0.96), 3, 1, PI / 2.);
                for i in 0..3 {
                    let a = i as f64 * 2. * PI / 3.;
                    circle(s, 0.64 * a.cos(), 0.64 * a.sin(), 0.16, 0.9);
                }
            }
            1 => {
                s.push_str("<path d='M .25 -.96 C -1.06 -.88 -1.06 .88 .25 .96 C -.48 .45 -.48 -.45 .25 -.96 Z M -.12 -.72 L .57 0 L -.12 .72 L .18 0 Z'/>");
                poly(s, 0.53, -0.38, 0.25, 4, 1, 0.);
                poly(s, 0.61, 0.48, 0.17, 4, 1, 0.);
            }
            2 => {
                let petals = 4 + seed % 5;
                for i in 0..petals {
                    let a = i as f64 * 360. / petals as f64;
                    let width = ink.between(0.25, 0.43);
                    let _ = write!(s, "<path transform='rotate({a:.3})' d='M 0 -.08 C -{width:.3} -.48 -{width:.3} -.8 0 -1 C {width:.3} -.8 {width:.3} -.48 0 -.08'/>");
                }
                poly(s, 0., 0., 0.47, petals as usize, 1, -PI / 2.);
            }
            3 => {
                s.push_str("<path d='M 0 -.93 L .2 -.38 C .32 -.83 .85 -.8 .92 -.4 C .76 -.57 .55 -.45 .47 -.12 L .8 .02 L .36 .2 L .5 .73 L .08 .48 L 0 .98 L -.08 .48 L -.5 .73 L -.36 .2 L -.8 .02 L -.47 -.12 C -.55 -.45 -.76 -.57 -.92 -.4 C -.85 -.8 -.32 -.83 -.2 -.38 Z M 0 -.7 L 0 .7 M -.45 -.22 L .45 .28 M .45 -.22 L -.45 .28'/>");
            }
            4 => {
                s.push_str("<path d='M -1 0 Q 0 -1.2 1 0 Q 0 1.2 -1 0 Z M 0 -.94 L .14 -.36 M 0 .94 L -.14 .36 M -.92 -.35 L -.63 -.07 M .92 .35 L .63 .07'/>");
                poly(s, 0., 0., ink.between(0.34, 0.48), 4, 1, 0.);
                arc(s, 0., 0., 0.68, PI * 1.15, PI * 0.7, 0.7);
                arc(s, 0., 0., 0.68, PI * 0.15, PI * 0.7, 0.7);
            }
            5 => {
                for i in 0..3 {
                    let a = i as f64 * 2. * PI / 3.;
                    poly(s, 0.37 * a.cos(), 0.37 * a.sin(), 0.63, 4, 1, a);
                }
                poly(s, 0., 0., 0.28, 3, 1, PI / 2.);
            }
            6 => {
                s.push_str("<path d='M 0 1 L 0 -.95 M -.17 -.73 L 0 -.95 L .17 -.73 M -.25 .98 L 0 .7 L .25 .98'/>");
                for i in 0..4 {
                    let y = -0.56 + i as f64 * 0.34;
                    let extent = ink.between(0.48, 0.92);
                    let _ = write!(s, "<path d='M -{extent:.3} {:.3} L 0 {y:.3} L {extent:.3} {:.3} M -{extent:.3} {y:.3} L -{extent:.3} {:.3} M {extent:.3} {y:.3} L {extent:.3} {:.3}'/>", y - 0.23, y - 0.23, y - 0.23, y - 0.23);
                }
            }
            7 => {
                s.push_str("<path d='M -.8 -.9 L .8 -.9 L -.8 .9 L .8 .9 Z M -.56 -.65 L .56 -.65 L -.56 .65 L .56 .65 Z M -1 0 L -.42 -.24 L 0 0 L .42 -.24 L 1 0 L .42 .24 L 0 0 L -.42 .24 Z'/>");
            }
            8 => {
                s.push_str("<g transform='scale(.8)'>");
                s.push_str("<path d='M -.95 -.95 L .95 -.95 L .95 .95 L -.95 .95 Z M -.74 -.74 L .74 -.74 L .74 .52 L -.52 .52 L -.52 -.52 L .52 -.52 L .52 .3 L -.3 .3 L -.3 -.3 L .3 -.3 M -.95 0 L -.74 0 M 0 .95 L 0 .52'/>");
                poly(s, 0., 0., 0.36, 4, 1, 0.);
                s.push_str("</g>");
            }
            9 => {
                let satellites = 3 + seed % 3;
                for i in 0..satellites {
                    let a = i as f64 * 2. * PI / satellites as f64;
                    circle(s, 0.64 * a.cos(), 0.64 * a.sin(), 0.28, 0.9);
                    poly(s, 0.64 * a.cos(), 0.64 * a.sin(), 0.17, 3, 1, a);
                }
                poly(
                    s,
                    0.,
                    0.,
                    0.74,
                    satellites as usize,
                    if satellites == 4 { 1 } else { 2 },
                    PI / 2.,
                );
            }
            10 => {
                let flames = 3 + seed % 4;
                for i in 0..flames {
                    let a = i as f64 * 360. / flames as f64;
                    let _ = write!(s, "<path transform='rotate({a:.3})' d='M -.13 -.16 C -.62 -.45 -.16 -.8 .26 -1 C -.08 -.5 .57 -.32 .14 -.07 C .33 -.37 .03 -.43 -.13 -.16 Z'/>");
                }
                poly(s, 0., 0., 0.32, flames as usize, 1, 0.);
            }
            _ => {
                for i in 0..3 {
                    let a = i as f64 * 120.;
                    let _ = write!(s, "<path transform='rotate({a:.3})' d='M 0 -.98 C .8 -.38 .69 .63 0 .48 C -.35 .4 -.2 -.08 .08 -.04 C .5 .01 .29 -.52 0 -.98 Z'/>");
                }
                poly(s, 0., 0., 0.2, 3, 1, -PI / 2.);
            }
        }
    }
    s.push_str("</g>");
}
fn design(s: &mut String, e: &Entity, x: f64, y: f64, r: f64, large: bool) {
    let seed = entity_seed(e);
    let rotation = (seed % 360) as f64 * PI / 180.;
    let mut ink = Entropy(seed.wrapping_add(71));
    // A calm circular collar contrasts with the wilder name-derived center.
    let _ = write!(
        s,
        "<g data-ring='1' stroke-width='{:.3}'>",
        (r * 0.004).max(0.85)
    );
    circle(s, x, y, r, 0.92);
    circle(s, x, y, r * 0.93, 0.65);
    circle(s, x, y, r * 0.73, 0.72);
    let repeats = match e.language.as_str() {
        "Rust" => 8,
        "Python" => 6,
        "JavaScript" => 12,
        "TypeScript" => 3,
        "Go" => 3,
        "C" => 12,
        "C++" => 16,
        "Java" => 4,
        _ => 8,
    };
    for i in 0..repeats {
        let angle = rotation + i as f64 * 2. * PI / repeats as f64;
        let degrees = angle * 180. / PI;
        let _ = write!(s, "<g transform='translate({x} {y}) rotate({degrees:.3}) scale({r})' stroke-width='.007'>");
        match e.language.as_str() {
            "Rust" => s.push_str("<path d='M -.024 -.94 L 0 -.99 L .024 -.94 M 0 -.99 L 0 -.90'/>"),
            "Python" => s.push_str("<path d='M -.035 -.965 Q 0 -.925 .035 -.965 Q 0 -.995 -.035 -.965 Z'/>"),
            "JavaScript" => s.push_str("<path d='M 0 -.99 L 0 -.94 M -.018 -.982 L .018 -.947 M .018 -.982 L -.018 -.947'/>"),
            "TypeScript" => {
                poly(s, 0., -0.87, 0.115, 3, 1, -PI / 2.);
                poly(s, 0., -0.87, 0.07, 3, 1, PI / 2.);
            }
            "Go" => s.push_str("<path d='M -.12 -.965 Q -.04 -.99 0 -.955 T .12 -.965 M -.1 -.938 Q -.03 -.965 .02 -.936 T .1 -.938'/>"),
            "C" => s.push_str("<path d='M -.03 -.94 L -.03 -.983 L .03 -.983 L .03 -.94'/>"),
            "C++" => {
                poly(s, -0.017, -0.964, 0.027, 4, 1, PI / 4.);
                poly(s, 0.017, -0.964, 0.027, 4, 1, PI / 4.);
            }
            "Java" => s.push_str("<path d='M -.028 -.94 L -.028 -.965 Q 0 -1.02 .028 -.965 L .028 -.94 M 0 -.99 L 0 -.94'/>"),
            _ => poly(s, 0., -0.964, 0.031, 4, 1, 0.),
        }
        s.push_str("</g>");
    }
    s.push_str("</g>");
    center_emblem(s, e, x, y, r);
    if large {
        // Offset satellites and partial orbits keep the overall inscription unruly.
        arc(
            s,
            x + r * 0.025,
            y - r * 0.02,
            r * 1.045,
            rotation,
            ink.between(2.2, 4.6),
            0.35,
        );
        for _ in 0..1 + seed % 3 {
            let a = ink.between(0., 2. * PI);
            let orbit = r * ink.between(0.58, 0.70);
            let sx = x + orbit * a.cos();
            let sy = y + orbit * a.sin();
            let small = (r * ink.between(0.085, 0.14)).min(22.);
            circle(s, sx, sy, small, 0.85);
            circle(s, sx, sy, small * 0.76, 0.55);
            poly(s, sx, sy, small * 0.5, 3 + (seed % 3) as usize, 1, a);
        }
    }
}
pub fn render(a: &Analysis, title: &str, min_lines: usize, palette: &str) -> Diagram {
    render_with_wingdings(a, title, min_lines, palette, false)
}
pub fn render_with_wingdings(
    a: &Analysis,
    title: &str,
    min_lines: usize,
    palette: &str,
    wingdings: bool,
) -> Diagram {
    let visible: Vec<_> = a
        .entities
        .iter()
        .filter(|e| e.kind != "merged_impl")
        .collect();
    let (placements, width, height) = scatter_layout(a, &visible);
    let (bg, ink, muted) = match palette {
        "parchment" => ("#f0e8d7", "#73502d", "#a59370"),
        "astral" => ("#090d20", "#8cd4ee", "#354968"),
        _ => ("#0c1112", "#d8be87", "#44514a"),
    };
    let outlines: BTreeMap<String, Outline> =
        serde_json::from_str(include_str!("glyphs.json")).expect("bundled glyph paths are valid");
    let _ = title; // Provenance lives in the recoverable archive, never in the artwork.
    let mut s=format!("<svg xmlns='http://www.w3.org/2000/svg' width='{width}' height='{height}' viewBox='0 0 {width} {height}'><defs><marker id='arrow' viewBox='0 0 10 10' refX='9' refY='5' markerWidth='7' markerHeight='7' orient='auto-start-reverse'><path d='M 0 0 L 10 5 L 0 10' fill='none' stroke='{ink}'/></marker></defs><rect width='100%' height='100%' fill='{bg}'/>");
    let mut atmosphere = Entropy(hash("wild grimoire") ^ visible.len() as u64);
    let _ = write!(s, "<g fill='{ink}' opacity='.17'>");
    for _ in 0..visible.len().saturating_mul(3).min(360) {
        let px = atmosphere.between(45., width as f64 - 45.);
        let py = atmosphere.between(125., height as f64 - 70.);
        let _ = write!(
            s,
            "<circle cx='{px:.1}' cy='{py:.1}' r='{:.1}'/>",
            atmosphere.between(0.7, 1.9)
        );
    }
    s.push_str("</g>");
    let _ = write!(s, "<g fill='none' stroke='{muted}' stroke-width='.7'>");
    for _ in 0..5 {
        arc(
            &mut s,
            width as f64 * 0.51,
            height as f64 * 0.53,
            atmosphere.between(
                width.min(height) as f64 * 0.2,
                width.max(height) as f64 * 0.6,
            ),
            atmosphere.between(0., 2. * PI),
            atmosphere.between(0.6, 2.8),
            0.21,
        );
    }
    s.push_str("</g>");
    // Channels are rendered behind seals. Endpoint trimming and arrows express caller -> callee.
    for edge in &a.edges {
        let Some(from) = placements.iter().find(|p| p.id == edge.from) else {
            continue;
        };
        let Some(to) = placements.iter().find(|p| p.id == edge.to) else {
            continue;
        };
        if edge.from == edge.to {
            let _=write!(s,"<path d='M {} {} C {} {}, {} {}, {} {}' fill='none' stroke='{ink}' opacity='.5' marker-end='url(#arrow)'/>",from.x-22.,from.y-from.radius,from.x-140.,from.y-from.radius-100.,from.x+140.,from.y-from.radius-100.,from.x+22.,from.y-from.radius);
            continue;
        }
        let dx = to.x - from.x;
        let dy = to.y - from.y;
        let distance = (dx * dx + dy * dy).sqrt();
        let ux = dx / distance;
        let uy = dy / distance;
        let (x1, y1, x2, y2) = (
            from.x + ux * (from.radius + 3.),
            from.y + uy * (from.radius + 3.),
            to.x - ux * (to.radius + 10.),
            to.y - uy * (to.radius + 10.),
        );
        let mut flow = Entropy(hash(&format!("{}:{}:{}", edge.from, edge.to, edge.kind)));
        let bend = flow.between(-0.3, 0.3) * distance + flow.between(-50., 50.);
        let c1x = x1 + (x2 - x1) * 0.31 - uy * bend;
        let c1y = y1 + (y2 - y1) * 0.31 + ux * bend;
        let c2x = x1 + (x2 - x1) * 0.68 + uy * bend * 0.48;
        let c2y = y1 + (y2 - y1) * 0.68 - ux * bend * 0.48;
        let _=write!(s,"<path d='M {x1:.1} {y1:.1} C {c1x:.1} {c1y:.1} {c2x:.1} {c2y:.1} {x2:.1} {y2:.1}' fill='none' stroke='{ink}' stroke-width='{}' opacity='{}' {} {}/>",if edge.kind=="contains"{0.8}else{1.3},if edge.kind=="contains"{0.26}else{0.55},if edge.kind=="contains"{"stroke-dasharray='4 7'"}else{""},if edge.kind=="calls"{"marker-end='url(#arrow)'"}else{""});
    }
    for place in &placements {
        let e = &a.entities[place.id];
        let (x, y, r) = (place.x, place.y, place.radius);
        let large = e.kind != "function" || e.lines >= min_lines;
        let _=write!(s,"<g id='entity-{}'><circle cx='{x}' cy='{y}' r='{}' fill='{bg}'/><g fill='none' stroke='{ink}' stroke-width='.85'>",e.id,r+2.);
        design(&mut s, e, x, y, r, large);
        let contour = encode_name(&e.qualified_name);
        let core = r * 0.52;
        let _=write!(s,"<polygon data-name-sigil='1' data-entity='{}' data-seal-radius='{r}' points='{}' transform='translate({x} {y}) scale({core})' stroke-width='.007' opacity='.65' stroke-linejoin='round'/> </g>",e.id,contour.svg_points());
        let mut script = Entropy(entity_seed(e).wrapping_add(17));
        let count = if large {
            34 + (entity_seed(e) % 35) as usize
        } else {
            15 + (entity_seed(e) % 13) as usize
        };
        let _ = write!(s, "<g fill='{ink}'>");
        for i in 0..count {
            let angle = i as f64 * 360. / count as f64 + script.between(-2.7, 2.7);
            let band = r * script.between(0.80, 0.87);
            let size = script
                .between(if large { 10. } else { 8. }, if large { 16. } else { 12. })
                .min(r * 0.28);
            let lean = script.between(-18., 18.);
            let token = e
                .tokens
                .get(i * e.tokens.len().max(1) / count)
                .map_or(e.name.as_str(), |x| x.as_str());
            let symbol = glyph(token);
            let _ = write!(s,"<g transform='translate({x} {y}) rotate({angle:.3}) translate(0 -{band:.1}) rotate({lean:.1})' opacity='.8'>");
            if wingdings && i % 3 == 0 {
                // Portable dingbat ornaments are vector geometry, not installed-font text.
                let points = polygon(0., 0., size * 0.43, 5, 2, PI / 2.);
                let _ = write!(
                    s,
                    "<polyline points='{points}' fill='none' stroke='{ink}' stroke-width='.7'/>"
                );
            } else {
                draw_outline(&mut s, &outlines, symbol, 0., 0., size);
            }
            s.push_str("</g>");
        }
        draw_outline(
            &mut s,
            &outlines,
            match e.kind.as_str() {
                "class" | "impl" => "♜",
                "enum" => "◈",
                "struct" => "⬡",
                "interface" => "△",
                "object" => "◇",
                _ => match e.language.as_str() {
                    "Rust" => "ᛟ",
                    "Python" => "☽",
                    "TypeScript" => "Δ",
                    "JavaScript" => "☉",
                    "Go" => "≋",
                    "Java" => "♜",
                    "C#" => "✧",
                    _ => "⚙",
                },
            },
            x,
            y,
            (if large { 26_f64 } else { 20_f64 }).min(r * 0.45),
        );
        s.push_str("</g></g>");
    }
    s.push_str("</svg>");
    Diagram {
        svg: s,
        width,
        height,
        placements,
        analysis: a.clone(),
    }
}

#[derive(Deserialize)]
struct Outline {
    path: String,
    cx: f64,
    cy: f64,
    units: f64,
}
fn draw_outline(
    s: &mut String,
    outlines: &BTreeMap<String, Outline>,
    symbol: &str,
    x: f64,
    y: f64,
    size: f64,
) {
    if let Some(o) = outlines.get(symbol) {
        let scale = size / o.units;
        let _=write!(s,"<path d='{}' transform='translate({x:.3} {y:.3}) scale({scale:.8} -{scale:.8}) translate({:.3} {:.3})'/>",o.path,-o.cx,-o.cy);
    } else {
        let _ = write!(
            s,
            "<path d='M {x} {} L {} {y} L {x} {} L {} {y} Z'/>",
            y - size * 0.3,
            x + size * 0.3,
            y + size * 0.3,
            x - size * 0.3
        );
    }
}
