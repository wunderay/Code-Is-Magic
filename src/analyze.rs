use crate::profiles::{contains, profile_for_language, profile_for_path, LanguageProfile};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Serialize, Deserialize)]
pub struct SourceFile {
    pub path: String,
    pub content: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Entity {
    pub id: usize,
    pub name: String,
    pub qualified_name: String,
    pub kind: String,
    pub path: String,
    pub language: String,
    pub line: usize,
    pub end_line: usize,
    pub lines: usize,
    pub owner: Option<usize>,
    pub tokens: Vec<String>,
    pub fields: Vec<String>,
    pub calls: Vec<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Edge {
    pub from: usize,
    pub to: usize,
    pub kind: String,
}
#[derive(Clone, Default, Debug, Serialize, Deserialize)]
pub struct Analysis {
    pub entities: Vec<Entity>,
    pub edges: Vec<Edge>,
    pub files: usize,
    pub bytes: usize,
    pub unresolved_calls: usize,
    pub warnings: Vec<String>,
}
#[derive(Clone, Debug)]
struct Token {
    text: String,
    line: usize,
    column: usize,
}

pub fn language(path: &str) -> &'static str {
    profile_for_path(path).map_or("C", |profile| profile.name.as_str())
}
pub fn supported(path: &str) -> bool {
    profile_for_path(path).is_some()
}
fn lex(source: &str, profile: &LanguageProfile) -> Vec<Token> {
    let chars: Vec<char> = source.chars().collect();
    let line_comment: Vec<char> = profile.line_comment.chars().collect();
    let (mut i, mut line, mut column) = (0, 1, 0);
    let mut tokens = Vec::new();
    while i < chars.len() {
        let c = chars[i];
        if c == '\n' {
            line += 1;
            column = 0;
            i += 1;
            continue;
        }
        if c.is_whitespace() {
            column += if c == '\t' { 4 } else { 1 };
            i += 1;
            continue;
        }
        if chars[i..].starts_with(&line_comment) {
            while i < chars.len() && chars[i] != '\n' {
                i += 1;
                column += 1;
            }
            continue;
        }
        if profile.block_comments && c == '/' && chars.get(i + 1) == Some(&'*') {
            i += 2;
            column += 2;
            let mut depth = 1;
            while i < chars.len() && depth > 0 {
                if chars[i] == '/' && chars.get(i + 1) == Some(&'*') {
                    depth += 1;
                    i += 2;
                    column += 2;
                } else if chars[i] == '*' && chars.get(i + 1) == Some(&'/') {
                    depth -= 1;
                    i += 2;
                    column += 2;
                } else {
                    if chars[i] == '\n' {
                        line += 1;
                        column = 0;
                    } else {
                        column += 1;
                    }
                    i += 1;
                }
            }
            continue;
        }
        let (start, start_line, start_col) = (i, line, column);
        // Delimited raw strings may contain ordinary quotes and code.
        let raw_prefix = if profile.raw_strings && c == 'r' {
            Some(i + 1)
        } else if profile.raw_strings && c == 'b' && chars.get(i + 1) == Some(&'r') {
            Some(i + 2)
        } else {
            None
        };
        if let Some(prefix) = raw_prefix {
            let mut quote = prefix;
            while chars.get(quote) == Some(&'#') {
                quote += 1;
            }
            if chars.get(quote) == Some(&'"') {
                let hashes = quote - prefix;
                column += quote + 1 - i;
                i = quote + 1;
                while i < chars.len() {
                    if chars[i] == '"' && (0..hashes).all(|h| chars.get(i + 1 + h) == Some(&'#')) {
                        i += 1 + hashes;
                        column += 1 + hashes;
                        break;
                    }
                    if chars[i] == '\n' {
                        line += 1;
                        column = 0;
                    } else {
                        column += 1;
                    }
                    i += 1;
                }
                tokens.push(Token {
                    text: "literal".into(),
                    line: start_line,
                    column: start_col,
                });
                continue;
            }
        }
        // A dialect may treat lifetime-like identifiers as identifiers, not strings.
        let lifetime = profile.lifetimes
            && c == '\''
            && chars.get(i + 1).is_some_and(|x| x.is_alphabetic())
            && chars.get(i + 2) != Some(&'\'');
        if matches!(c, '"' | '\'' | '`') && !lifetime {
            let triple = profile.triple_quotes
                && chars.get(i + 1) == Some(&c)
                && chars.get(i + 2) == Some(&c);
            i += if triple { 3 } else { 1 };
            column += if triple { 3 } else { 1 };
            while i < chars.len() {
                if chars[i] == '\\' {
                    i = (i + 2).min(chars.len());
                    column += 2;
                    continue;
                }
                if chars[i] == c
                    && (!triple || (chars.get(i + 1) == Some(&c) && chars.get(i + 2) == Some(&c)))
                {
                    i += if triple { 3 } else { 1 };
                    column += if triple { 3 } else { 1 };
                    break;
                }
                if chars[i] == '\n' {
                    line += 1;
                    column = 0;
                } else {
                    column += 1;
                }
                i += 1;
            }
            tokens.push(Token {
                text: "literal".into(),
                line: start_line,
                column: start_col,
            });
        } else if c.is_alphanumeric() || c == '_' || lifetime {
            i += 1;
            column += 1;
            while i < chars.len() && (chars[i].is_alphanumeric() || chars[i] == '_') {
                i += 1;
                column += 1;
            }
            tokens.push(Token {
                text: chars[start..i].iter().collect(),
                line: start_line,
                column: start_col,
            });
        } else {
            let pair: String = chars[i..(i + 2).min(chars.len())].iter().collect();
            let n = if matches!(
                pair.as_str(),
                "=>" | "->" | "::" | "==" | "!=" | "<=" | ">=" | "&&" | "||" | "+="
            ) {
                2
            } else {
                1
            };
            tokens.push(Token {
                text: chars[i..i + n].iter().collect(),
                line: start_line,
                column: start_col,
            });
            i += n;
            column += n;
        }
    }
    tokens
}
fn ident(s: &str) -> bool {
    s.chars()
        .next()
        .is_some_and(|c| c.is_alphabetic() || c == '_')
        && s != "literal"
}
fn control(s: &str, profile: &LanguageProfile) -> bool {
    contains(&profile.controls, s)
}
fn pairs(t: &[Token]) -> BTreeMap<usize, usize> {
    let mut stack = Vec::new();
    let mut map = BTreeMap::new();
    for (i, x) in t.iter().enumerate() {
        match x.text.as_str() {
            "(" | "{" | "[" => stack.push(i),
            ")" | "}" | "]" => {
                if let Some(&j) = stack.last() {
                    if matches!(
                        (t[j].text.as_str(), x.text.as_str()),
                        ("(", ")") | ("{", "}") | ("[", "]")
                    ) {
                        stack.pop();
                        map.insert(j, i);
                    }
                }
            }
            _ => {}
        }
    }
    map
}
struct Range {
    id: usize,
    start: usize,
    body: usize,
    end: usize,
}
fn add_entity(
    a: &mut Analysis,
    ranges: &mut Vec<Range>,
    file: &SourceFile,
    t: &[Token],
    kind: &str,
    span: [usize; 4],
) {
    let [name, start, body, end] = span;
    if end <= body
        || ranges
            .iter()
            .any(|r| r.start == start && a.entities[r.id].kind == kind)
    {
        return;
    }
    let id = a.entities.len();
    let lines = t[body..=end]
        .iter()
        .map(|x| x.line)
        .collect::<BTreeSet<_>>()
        .len();
    let fields = Vec::new();
    a.entities.push(Entity {
        id,
        name: t[name].text.clone(),
        qualified_name: t[name].text.clone(),
        kind: kind.into(),
        path: file.path.clone(),
        language: language(&file.path).into(),
        line: t[start].line,
        end_line: t[end].line,
        lines,
        owner: None,
        tokens: Vec::new(),
        fields,
        calls: Vec::new(),
    });
    ranges.push(Range {
        id,
        start,
        body,
        end,
    });
}
pub fn analyze(files: &[SourceFile]) -> Analysis {
    let mut a = Analysis::default();
    let mut ordered_files: Vec<_> = files.iter().collect();
    ordered_files.sort_by(|left, right| left.path.cmp(&right.path));
    for file in ordered_files {
        if !supported(&file.path) {
            a.warnings
                .push(format!("Unsupported source: {}", file.path));
            continue;
        }
        a.files += 1;
        a.bytes += file.content.len();
        let profile = profile_for_path(&file.path).expect("supported source profile");
        let indented = !profile.indented_declarations.is_empty();
        let t = lex(&file.content, profile);
        let p = pairs(&t);
        let mut ranges = Vec::new();
        for i in 0..t.len() {
            let word = t[i].text.as_str();
            if indented
                && profile.indented_declarations.contains_key(word)
                && t.get(i + 1).is_some_and(|x| ident(&x.text))
            {
                let mut colon = i + 2;
                while colon < t.len() && t[colon].text != ":" {
                    if let Some(end) = p.get(&colon) {
                        colon = *end;
                    }
                    colon += 1;
                }
                if colon >= t.len() {
                    continue;
                }
                let indent = t
                    .iter()
                    .find(|x| x.line == t[i].line)
                    .map_or(t[i].column, |x| x.column);
                let header_line = t[colon].line;
                let mut end = colon;
                for (j, token) in t.iter().enumerate().skip(colon + 1) {
                    if token.line > header_line && token.column <= indent {
                        break;
                    }
                    end = j;
                }
                add_entity(
                    &mut a,
                    &mut ranges,
                    file,
                    &t,
                    profile.indented_declarations[word].as_str(),
                    [i + 1, i, colon, end],
                );
                continue;
            }
            if !indented && profile.braced_declarations.contains_key(word) {
                let mut n = i + 1;
                if t.get(n).is_some_and(|x| x.text == "<") {
                    while n < t.len() && t[n].text != ">" {
                        n += 1;
                    }
                    n += 1;
                }
                if !t.get(n).is_some_and(|x| ident(&x.text)) {
                    continue;
                }
                let mut body = n + 1;
                while body < t.len() && !matches!(t[body].text.as_str(), "{" | ";") {
                    body += 1;
                }
                if let Some(end) = p.get(&body) {
                    let kind = profile.braced_declarations[word].as_str();
                    add_entity(&mut a, &mut ranges, file, &t, kind, [n, i, body, *end]);
                }
            }
            if !indented
                && word == profile.type_keyword
                && t.get(i + 1).is_some_and(|x| ident(&x.text))
            {
                if let Some(body) = (i + 2..t.len())
                    .take_while(|&j| t[j].text != ";")
                    .find(|&j| t[j].text == "{")
                {
                    if let Some(end) = p.get(&body) {
                        let kind = t
                            .get(i + 2)
                            .and_then(|token| profile.type_kinds.get(&token.text))
                            .unwrap_or(&profile.type_default_kind)
                            .as_str();
                        add_entity(&mut a, &mut ranges, file, &t, kind, [i + 1, i, body, *end]);
                    }
                }
            }
            // Named object literals get their own structural frame.
            if !indented
                && contains(&profile.object_keywords, word)
                && t.get(i + 2).is_some_and(|x| x.text == "=")
                && t.get(i + 3).is_some_and(|x| x.text == "{")
            {
                if let Some(end) = p.get(&(i + 3)) {
                    add_entity(
                        &mut a,
                        &mut ranges,
                        file,
                        &t,
                        "object",
                        [i + 1, i, i + 3, *end],
                    );
                }
            }
            if indented {
                continue;
            }
            if word == "(" {
                let Some(&close) = p.get(&i) else {
                    continue;
                };
                // Skip a dialect's receiver signature before considering its method signature.
                if i > 0 && t[i - 1].text == profile.receiver_keyword {
                    continue;
                }
                let mut name = i.saturating_sub(1);
                if t.get(name).is_some_and(|x| x.text == ">") {
                    while name > 0 && t[name].text != "<" {
                        name -= 1;
                    }
                    name = name.saturating_sub(1);
                }
                if !ident(&t[name].text) || control(&t[name].text, profile) {
                    continue;
                }
                // Some dialects require an explicit function declaration keyword.
                if !profile.required_function_keyword.is_empty()
                    && (name == 0 || t[name - 1].text != profile.required_function_keyword)
                {
                    continue;
                }
                let mut body = close + 1;
                while body < t.len()
                    && !matches!(t[body].text.as_str(), "{" | ";" | "=" | "=>" | "}")
                {
                    // Function signatures may contain return types, but not a second call.
                    if t[body].text == "(" {
                        break;
                    }
                    body += 1;
                }
                if let Some(&end) = p.get(&body).filter(|_| t[body].text == "{") {
                    // A call followed by an object/block is not a declaration.
                    if name > 0 && contains(&profile.reject_function_prefixes, &t[name - 1].text) {
                        continue;
                    }
                    add_entity(
                        &mut a,
                        &mut ranges,
                        file,
                        &t,
                        "function",
                        [name, name, body, end],
                    );
                }
            }
            if word == profile.arrow_token {
                let body = i + 1;
                let Some(&end) = p.get(&body).filter(|_| t[body].text == "{") else {
                    continue;
                };
                let lower = i.saturating_sub(80);
                if let Some(eq) = (lower..i)
                    .rev()
                    .take_while(|&j| !matches!(t[j].text.as_str(), ";" | "{" | "}"))
                    .find(|&j| t[j].text == "=")
                {
                    if eq > 0 && ident(&t[eq - 1].text) {
                        add_entity(
                            &mut a,
                            &mut ranges,
                            file,
                            &t,
                            "function",
                            [eq - 1, eq - 1, body, end],
                        );
                    }
                }
            }
        }
        for range in &ranges {
            let id = range.id;
            let owner = ranges
                .iter()
                .filter(|r| {
                    r.start < range.start
                        && r.end >= range.end
                        && a.entities[r.id].kind != "function"
                })
                .min_by_key(|r| r.end - r.start)
                .map(|r| r.id);
            a.entities[id].owner = owner;
            if let Some(o) = owner {
                a.entities[id].qualified_name =
                    format!("{}::{}", a.entities[o].name, a.entities[id].name);
            }
            let mut calls = BTreeSet::new();
            for j in range.body + 1..range.end {
                if ranges
                    .iter()
                    .any(|r| r.id != id && r.start > range.start && j >= r.start && j <= r.end)
                {
                    continue;
                }
                a.entities[id].tokens.push(t[j].text.clone());
                if t.get(j + 1).is_some_and(|x| x.text == "(")
                    && ident(&t[j].text)
                    && !control(&t[j].text, profile)
                {
                    let call = if j >= 2 && contains(&profile.member_operators, &t[j - 1].text) {
                        format!("{}::{}", t[j - 2].text, t[j].text)
                    } else {
                        t[j].text.clone()
                    };
                    calls.insert(call);
                }
            }
            a.entities[id].calls = calls.into_iter().collect();
            if a.entities[id].kind != "function" {
                a.entities[id].fields = a.entities[id]
                    .tokens
                    .windows(2)
                    .filter(|w| ident(&w[0]) && matches!(w[1].as_str(), ":" | ";" | "=" | ","))
                    .map(|w| w[0].clone())
                    .collect::<BTreeSet<_>>()
                    .into_iter()
                    .take(24)
                    .collect();
            }
        }
    }
    // Merge configured extension containers into their declared type when it is local.
    let mut aliases = BTreeMap::new();
    for e in &a.entities {
        if contains(&profile_for_language(&e.language).merge_containers, &e.kind) {
            if let Some(base) = a.entities.iter().find(|b| {
                b.name == e.name
                    && b.path == e.path
                    && !contains(&profile_for_language(&b.language).merge_containers, &b.kind)
                    && b.kind != "function"
            }) {
                aliases.insert(e.id, base.id);
            }
        }
    }
    for e in &mut a.entities {
        if let Some(o) = e.owner {
            if let Some(&b) = aliases.get(&o) {
                e.owner = Some(b);
            }
        }
    }
    let mut edge_keys = BTreeSet::new();
    for e in &a.entities {
        if let Some(o) = e.owner {
            edge_keys.insert((o, e.id, "contains".to_string()));
        }
        if e.kind != "function" {
            continue;
        }
        for call in &e.calls {
            let parts: Vec<_> = call.split("::").collect();
            let name = parts.last().unwrap();
            let candidates: Vec<_> = a
                .entities
                .iter()
                .filter(|b| {
                    b.kind == "function"
                        && b.name == *name
                        && profile_for_language(&b.language).call_group
                            == profile_for_language(&e.language).call_group
                })
                .collect();
            let target = if parts.len() > 1 {
                let qualifier = parts[0];
                let qualified: Vec<_> = candidates
                    .iter()
                    .copied()
                    .filter(|b| {
                        if contains(
                            &profile_for_language(&e.language).self_qualifiers,
                            qualifier,
                        ) {
                            b.owner == e.owner && e.owner.is_some()
                        } else {
                            b.owner.is_some_and(|o| a.entities[o].name == qualifier)
                        }
                    })
                    .collect();
                if qualified.len() == 1 {
                    Some(qualified[0])
                } else {
                    None
                }
            } else {
                let local: Vec<_> = candidates
                    .iter()
                    .copied()
                    .filter(|b| b.path == e.path && b.owner == e.owner)
                    .collect();
                if local.len() == 1 {
                    Some(local[0])
                } else if candidates.len() == 1 {
                    Some(candidates[0])
                } else {
                    None
                }
            };
            if let Some(target) = target {
                edge_keys.insert((e.id, target.id, "calls".into()));
            } else {
                a.unresolved_calls += 1;
            }
        }
    }
    a.edges = edge_keys
        .into_iter()
        .map(|(from, to, kind)| Edge { from, to, kind })
        .collect();
    // Keep IDs stable; renderer hides merged impl containers.
    for id in aliases.keys() {
        a.entities[*id].kind = "merged_impl".into();
    }
    a
}
