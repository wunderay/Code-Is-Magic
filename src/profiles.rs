//! Portable language policies emitted by native-language authoring modules.
//! Runtime source code is never executed; the same immutable policies serve native and WASM.
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, sync::OnceLock};

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LanguageProfile {
    pub id: String,
    pub name: String,
    pub extensions: Vec<String>,
    pub call_group: String,
    pub line_comment: String,
    pub block_comments: bool,
    pub raw_strings: bool,
    pub lifetimes: bool,
    pub triple_quotes: bool,
    pub indented_declarations: BTreeMap<String, String>,
    pub braced_declarations: BTreeMap<String, String>,
    pub type_keyword: String,
    pub type_kinds: BTreeMap<String, String>,
    pub type_default_kind: String,
    pub object_keywords: Vec<String>,
    pub receiver_keyword: String,
    pub required_function_keyword: String,
    pub arrow_token: String,
    pub controls: Vec<String>,
    pub reject_function_prefixes: Vec<String>,
    pub member_operators: Vec<String>,
    pub self_qualifiers: Vec<String>,
    pub merge_containers: Vec<String>,
    pub sigil: String,
    pub color: String,
    pub repeats: usize,
    pub collar: Vec<CollarOperation>,
}
#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "op", rename_all = "snake_case", deny_unknown_fields)]
pub enum CollarOperation {
    Path {
        data: String,
    },
    Polygon {
        cx: f64,
        cy: f64,
        r: f64,
        n: usize,
        step: usize,
        rotation_degrees: f64,
    },
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Registry {
    version: u32,
    profiles: Vec<LanguageProfile>,
}
pub fn language_profiles() -> &'static [LanguageProfile] {
    static REGISTRY: OnceLock<Registry> = OnceLock::new();
    &REGISTRY
        .get_or_init(|| {
            let registry: Registry =
                serde_json::from_str(include_str!("../languages/profiles.json"))
                    .expect("validated bundled language profiles");
            assert_eq!(registry.version, 1, "unsupported language profile version");
            registry
        })
        .profiles
}
pub fn profile_for_path(path: &str) -> Option<&'static LanguageProfile> {
    let extension = path.rsplit('.').next()?;
    language_profiles()
        .iter()
        .find(|profile| profile.extensions.iter().any(|x| x == extension))
}
pub fn profile_for_language(name: &str) -> &'static LanguageProfile {
    let profiles = language_profiles();
    profiles
        .iter()
        .find(|profile| profile.name == name)
        .unwrap_or_else(|| {
            profiles
                .iter()
                .find(|profile| profile.id == "c")
                .expect("bundled base profile")
        })
}
pub fn contains(values: &[String], value: &str) -> bool {
    values.iter().any(|item| item == value)
}
