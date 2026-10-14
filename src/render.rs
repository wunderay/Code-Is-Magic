use crate::{Analysis, Entity};
use base64::Engine;
use serde::Serialize;
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
pub fn escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}
fn hash(s: &str) -> u64 {
    s.bytes().fold(14695981039346656037, |h, b| {
        (h ^ b as u64).wrapping_mul(1099511628211)
    })
}
fn polygon(cx: f64, cy: f64, r: f64, n: usize, step: usize, rotation: f64) -> String {
    (0..=n)
        .map(|i| {
            let a = rotation + (i * step % n) as f64 * 2.0 * PI / n as f64;
            format!("{:.1},{:.1}", cx + r * a.cos(), cy + r * a.sin())
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
        "<circle cx='{x:.1}' cy='{y:.1}' r='{r:.1}' opacity='{opacity}'/>"
    );
}
fn poly(s: &mut String, x: f64, y: f64, r: f64, n: usize, step: usize, rot: f64) {
    let _ = write!(s, "<polyline points='{}'/>", polygon(x, y, r, n, step, rot));
}
fn design(s: &mut String, e: &Entity, x: f64, y: f64, r: f64, large: bool) {
    let seed = hash(&format!("{}{}", e.path, e.name));
    let rotation = (seed % 360) as f64 * PI / 180.;
    circle(s, x, y, r, 0.75);
    circle(s, x, y, r - 5., 0.3);
    if e.kind != "function" {
        match e.kind.as_str() {
            "class" | "impl" => {
                poly(s, x, y, r * 0.70, 4, 1, PI / 4.);
                poly(s, x, y, r * 0.56, 4, 1, PI / 4.);
                poly(s, x, y, r * 0.85, 8, 1, PI / 8.);
            }
            "enum" => {
                poly(s, x, y, r * 0.72, 4, 1, 0.);
                poly(s, x, y, r * 0.65, 4, 1, PI / 4.);
            }
            "object" => {
                poly(s, x, y, r * 0.7, 6, 1, PI / 6.);
                for i in 0..3 {
                    let a = PI / 6. + i as f64 * 2. * PI / 3.;
                    let _ = write!(
                        s,
                        "<path d='M {x} {y} L {} {}'/>",
                        x + r * 0.7 * a.cos(),
                        y + r * 0.7 * a.sin()
                    );
                }
            }
            "interface" => {
                poly(s, x, y, r * 0.74, 3, 1, -PI / 2.);
                poly(s, x, y, r * 0.65, 3, 1, PI / 2.);
            }
            _ => {
                poly(s, x, y, r * 0.77, 6, 1, 0.);
                poly(s, x, y, r * 0.66, 6, 1, 0.);
            }
        }
    }
    let inner = if large { r * 0.67 } else { r * 0.70 };
    match e.language.as_str() {
        "Rust" => {
            poly(s, x, y, inner, 8, 3, rotation);
            poly(s, x, y, inner * 0.74, 4, 1, rotation);
        }
        "Python" => {
            circle(s, x - inner * 0.20, y, inner * 0.65, 0.55);
            circle(s, x + inner * 0.20, y, inner * 0.65, 0.55);
            let mut d = String::new();
            for i in 0..100 {
                let a = i as f64 * 0.14;
                let rr = inner * 0.8 * i as f64 / 100.;
                let _ = write!(
                    d,
                    "{} {:.1} {:.1} ",
                    if i == 0 { "M" } else { "L" },
                    x + rr * a.cos(),
                    y + rr * a.sin()
                );
            }
            let _ = write!(s, "<path d='{d}' opacity='.45'/>");
        }
        "JavaScript" => {
            circle(s, x, y, inner * 0.75, 0.5);
            poly(s, x, y, inner, 12, 5, rotation);
        }
        "TypeScript" => {
            poly(s, x, y, inner, 3, 1, -PI / 2.);
            poly(s, x, y, inner * 0.9, 3, 1, PI / 2.);
            poly(s, x, y, inner * 0.65, 6, 1, 0.);
        }
        "Go" => {
            for i in 0..3 {
                let a = i as f64 * 2. * PI / 3. + rotation;
                circle(
                    s,
                    x + inner * 0.3 * a.cos(),
                    y + inner * 0.3 * a.sin(),
                    inner * 0.57,
                    0.6,
                );
            }
            poly(s, x, y, inner, 3, 1, rotation);
        }
        "Java" => {
            poly(s, x, y, inner, 4, 1, PI / 4.);
            poly(s, x, y, inner, 4, 1, 0.);
            for i in 0..4 {
                let a = i as f64 * PI / 2.;
                circle(
                    s,
                    x + inner * 0.65 * a.cos(),
                    y + inner * 0.65 * a.sin(),
                    inner * 0.24,
                    0.55,
                );
            }
        }
        "C#" => {
            poly(s, x, y, inner, 4, 1, 0.);
            poly(s, x, y, inner * 0.7, 4, 1, 0.);
            poly(s, x, y, inner, 8, 3, PI / 8.);
        }
        _ => {
            poly(
                s,
                x,
                y,
                inner,
                if e.language == "C++" { 16 } else { 12 },
                1,
                rotation,
            );
            circle(s, x, y, inner * 0.66, 0.6);
        }
    }
    let teeth = if large { 32 } else { 12 };
    for i in 0..teeth {
        let a = i as f64 * 2. * PI / teeth as f64;
        let rr = r - 10.;
        let _ = write!(
            s,
            "<path d='M {:.1} {:.1} L {:.1} {:.1}' opacity='.4'/>",
            x + rr * a.cos(),
            y + rr * a.sin(),
            x + (rr - 4.) * a.cos(),
            y + (rr - 4.) * a.sin()
        );
    }
    circle(s, x, y, if large { r * 0.26 } else { r * 0.3 }, 0.55);
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
    let cols = (visible.len().max(1) as f64).sqrt().ceil() as usize;
    let rows = visible.len().max(1).div_ceil(cols);
    let width = (cols * 340 + 160).max(960) as u32;
    let height = (rows * 340 + 240).max(720) as u32;
    let mut placements = Vec::new();
    for (i, e) in visible.iter().enumerate() {
        placements.push(Placement {
            id: e.id,
            x: 120. + (i % cols) as f64 * 340. + 85.,
            y: 170. + (i / cols) as f64 * 340. + 70.,
            radius: if e.kind == "function" && e.lines < min_lines {
                66.
            } else {
                118.
            },
        });
    }
    let (bg, ink, muted) = match palette {
        "parchment" => ("#f0e8d7", "#73502d", "#a59370"),
        "astral" => ("#090d20", "#8cd4ee", "#354968"),
        _ => ("#0c1112", "#d8be87", "#44514a"),
    };
    let fonts = embedded_fonts();
    let mut s=format!("<svg xmlns='http://www.w3.org/2000/svg' width='{width}' height='{height}' viewBox='0 0 {width} {height}'><defs>{fonts}<pattern id='grid' width='40' height='40' patternUnits='userSpaceOnUse'><path d='M 40 0 L 0 0 0 40' fill='none' stroke='{muted}' stroke-width='.4' opacity='.25'/></pattern><marker id='arrow' viewBox='0 0 10 10' refX='9' refY='5' markerWidth='7' markerHeight='7' orient='auto-start-reverse'><path d='M 0 0 L 10 5 L 0 10' fill='none' stroke='{ink}'/></marker></defs><rect width='100%' height='100%' fill='{bg}'/><rect width='100%' height='100%' fill='url(#grid)'/><g fill='none' stroke='{muted}'><rect x='22' y='22' width='{}' height='{}'/><rect x='28' y='28' width='{}' height='{}'/></g><g fill='{ink}' font-family='Georgia,serif'><text x='54' y='66' font-size='22' letter-spacing='4'>CODE IS MAGIC</text><text x='54' y='91' font-size='12' opacity='.6'>{}</text></g>",width-44,height-44,width-56,height-56,escape(title));
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
        let bend = if edge.kind == "contains" { 0. } else { 24. };
        let _=write!(s,"<path d='M {x1:.1} {y1:.1} Q {:.1} {:.1} {x2:.1} {y2:.1}' fill='none' stroke='{ink}' stroke-width='{}' opacity='{}' {} {}/>",(x1+x2)/2.-uy*bend,(y1+y2)/2.+ux*bend,if edge.kind=="contains"{0.8}else{1.3},if edge.kind=="contains"{0.22}else{0.48},if edge.kind=="contains"{"stroke-dasharray='4 7'"}else{""},if edge.kind=="calls"{"marker-end='url(#arrow)'"}else{""});
    }
    for place in &placements {
        let e = &a.entities[place.id];
        let (x, y, r) = (place.x, place.y, place.radius);
        let large = r > 100.;
        let _=write!(s,"<g id='entity-{}'><title>{} · {}:{} · {} · {} source lines</title><circle cx='{x}' cy='{y}' r='{}' fill='{bg}'/><g fill='none' stroke='{ink}' stroke-width='.85'>",e.id,escape(&e.qualified_name),escape(&e.path),e.line,e.language,e.lines,r+2.);
        design(&mut s, e, x, y, r, large);
        s.push_str("</g>");
        let count = if large { 44 } else { 20 };
        let _=write!(s,"<g fill='{ink}' font-family='CIMRunic,CIMSymbols,CIMGreek,DejaVu Sans,Segoe UI Symbol,serif' font-size='{}' text-anchor='middle'>",if large{13}else{10});
        for i in 0..count {
            let angle = i as f64 * 360. / count as f64;
            let token = e
                .tokens
                .get(i * e.tokens.len().max(1) / count)
                .map_or(e.name.as_str(), |x| x.as_str());
            let use_wingdings = wingdings && i % 3 == 0;
            let font = if use_wingdings {
                "Wingdings"
            } else {
                "CIMRunic,CIMSymbols,CIMGreek,DejaVu Sans,Segoe UI Symbol,serif"
            };
            let symbol = if use_wingdings {
                const WING: [&str; 12] =
                    ["J", "K", "N", "Y", "a", "b", "l", "n", "p", "v", "z", "u"];
                WING[(hash(token) % WING.len() as u64) as usize]
            } else {
                glyph(token)
            };
            let _=write!(s,"<text transform='translate({x} {y}) rotate({angle}) translate(0 -{})' opacity='.8' font-family='{font}'>{symbol}</text>",r-20.);
        }
        let _ = write!(
            s,
            "<text x='{x}' y='{}' font-family='CIMRunic,CIMSymbols,CIMGreek,Georgia,serif' font-size='{}'>{}</text></g>",
            y + 7.,
            if large { 26 } else { 20 },
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
            }
        );
        let label = if e.qualified_name.chars().count() > 32 {
            format!("{}…", e.qualified_name.chars().take(30).collect::<String>())
        } else {
            e.qualified_name.clone()
        };
        let _=write!(s,"<g fill='{ink}' text-anchor='middle' font-family='ui-monospace,monospace'><text x='{x}' y='{}' font-size='12'>{}</text><text x='{x}' y='{}' font-size='9' opacity='.5'>{} · {} · L{}</text></g></g>",y+r+24.,escape(&label),y+r+41.,e.language,e.kind,e.line);
    }
    if visible.is_empty() {
        let _=write!(s,"<text x='{}' y='{}' fill='{ink}' text-anchor='middle' font-family='Georgia,serif' font-size='24'>No supported declarations found</text>",width/2,height/2);
    }
    let _=write!(s,"<text x='54' y='{}' fill='{ink}' opacity='.5' font-family='monospace' font-size='10'>{} source files · {} seals · {} call channels · structural analysis</text></svg>",height-48,a.files,visible.len(),a.edges.iter().filter(|e|e.kind=="calls").count());
    Diagram {
        svg: s,
        width,
        height,
        placements,
        analysis: a.clone(),
    }
}

fn embedded_fonts() -> String {
    let mut css = String::from("<style>");
    for (name, bytes) in [
        (
            "CIMRunic",
            include_bytes!("../web/fonts/NotoSansRunic-Regular.ttf").as_slice(),
        ),
        (
            "CIMSymbols",
            include_bytes!("../web/fonts/NotoSansSymbols2-Regular.ttf").as_slice(),
        ),
        (
            "CIMGreek",
            include_bytes!("../web/fonts/NotoSans-Regular.ttf").as_slice(),
        ),
    ] {
        let data = base64::engine::general_purpose::STANDARD.encode(bytes);
        let _ = write!(css, "@font-face{{font-family:'{name}';src:url('data:font/ttf;base64,{data}') format('truetype');}}");
    }
    css.push_str("</style>");
    css
}
