mod analyze;
mod archive;
mod crypto;
pub use crypto::MagicKey;
mod profiles;
mod render;
mod sigil;
pub use analyze::{analyze, supported, Analysis, Edge, Entity, SourceFile};
pub use archive::{
    decode_png, decode_png_with_key, decode_svg, decode_svg_with_key, embed_svg,
    embed_svg_with_key, encode_png, encode_png_with_key, snapshot_zip, ArchiveEntry, Capsule,
    Snapshot,
};
pub use profiles::{language_profiles, profile_for_language, profile_for_path, LanguageProfile};
pub use render::{
    render, render_with_key, render_with_options, render_with_wingdings, Diagram, Placement,
};
pub use sigil::{
    decode_name, decode_name_with_key, decode_svg_names, decode_svg_names_with_key, encode_name,
    encode_name_with_key, DecodedName, NameSigil,
};
use wasm_bindgen::prelude::*;

/// Browser and CLI use exactly the same analysis and renderer.
#[wasm_bindgen]
pub fn transmute(
    files_json: &str,
    title: &str,
    min_lines: usize,
    palette: &str,
    wingdings: bool,
) -> Result<String, JsValue> {
    transmute_with_options(files_json, title, min_lines, palette, wingdings, false)
}

#[wasm_bindgen]
pub fn transmute_with_options(
    files_json: &str,
    title: &str,
    min_lines: usize,
    palette: &str,
    wingdings: bool,
    language_colors: bool,
) -> Result<String, JsValue> {
    let files: Vec<SourceFile> =
        serde_json::from_str(files_json).map_err(|e| JsValue::from_str(&e.to_string()))?;
    let analysis = analyze(&files);
    serde_json::to_string(&render_with_options(
        &analysis,
        title,
        min_lines.max(1),
        palette,
        wingdings,
        language_colors,
    ))
    .map_err(|e| JsValue::from_str(&e.to_string()))
}

#[wasm_bindgen]
pub fn pack_png(capsule_json: &str) -> Result<Vec<u8>, JsValue> {
    let capsule: Capsule =
        serde_json::from_str(capsule_json).map_err(|e| JsValue::from_str(&e.to_string()))?;
    encode_png(&capsule).map_err(|e| JsValue::from_str(&e))
}
#[wasm_bindgen]
pub fn pack_svg(capsule_json: &str) -> Result<String, JsValue> {
    let capsule: Capsule =
        serde_json::from_str(capsule_json).map_err(|e| JsValue::from_str(&e.to_string()))?;
    embed_svg(&capsule).map_err(|e| JsValue::from_str(&e))
}
#[wasm_bindgen]
pub fn recover_image(bytes: &[u8]) -> Result<String, JsValue> {
    let capsule = if bytes.starts_with(b"\x89PNG") {
        decode_png(bytes)
    } else {
        std::str::from_utf8(bytes)
            .map_err(|_| "Not a source-carrying PNG or SVG".to_string())
            .and_then(decode_svg)
    }
    .map_err(|e| JsValue::from_str(&e))?;
    serde_json::to_string(&capsule).map_err(|e| JsValue::from_str(&e.to_string()))
}
#[wasm_bindgen]
pub fn recover_zip(snapshot_json: &str) -> Result<Vec<u8>, JsValue> {
    let snapshot: Snapshot =
        serde_json::from_str(snapshot_json).map_err(|e| JsValue::from_str(&e.to_string()))?;
    snapshot_zip(&snapshot).map_err(|e| JsValue::from_str(&e))
}
#[wasm_bindgen]
pub fn recover_names(svg: &str) -> Result<String, JsValue> {
    let names = decode_svg_names(svg).map_err(|e| JsValue::from_str(&e))?;
    serde_json::to_string(&names).map_err(|e| JsValue::from_str(&e.to_string()))
}

fn optional_key(text: &str) -> Result<Option<MagicKey>, JsValue> {
    if text.is_empty() {
        Ok(None)
    } else {
        MagicKey::from_hex(text)
            .map(Some)
            .map_err(|e| JsValue::from_str(&e))
    }
}
#[wasm_bindgen]
#[allow(clippy::too_many_arguments)]
pub fn transmute_keyed(
    files_json: &str,
    title: &str,
    min_lines: usize,
    palette: &str,
    wingdings: bool,
    language_colors: bool,
    key_hex: &str,
) -> Result<String, JsValue> {
    let key = optional_key(key_hex)?;
    let files: Vec<SourceFile> =
        serde_json::from_str(files_json).map_err(|e| JsValue::from_str(&e.to_string()))?;
    let diagram = render_with_key(
        &analyze(&files),
        title,
        min_lines.max(1),
        palette,
        wingdings,
        language_colors,
        key.as_ref(),
    )
    .map_err(|e| JsValue::from_str(&e))?;
    serde_json::to_string(&diagram).map_err(|e| JsValue::from_str(&e.to_string()))
}
#[wasm_bindgen]
pub fn pack_png_keyed(capsule_json: &str, key_hex: &str) -> Result<Vec<u8>, JsValue> {
    let key = optional_key(key_hex)?;
    let capsule: Capsule =
        serde_json::from_str(capsule_json).map_err(|e| JsValue::from_str(&e.to_string()))?;
    encode_png_with_key(&capsule, key.as_ref()).map_err(|e| JsValue::from_str(&e))
}
#[wasm_bindgen]
pub fn pack_svg_keyed(capsule_json: &str, key_hex: &str) -> Result<String, JsValue> {
    let key = optional_key(key_hex)?;
    let capsule: Capsule =
        serde_json::from_str(capsule_json).map_err(|e| JsValue::from_str(&e.to_string()))?;
    embed_svg_with_key(&capsule, key.as_ref()).map_err(|e| JsValue::from_str(&e))
}
#[wasm_bindgen]
pub fn recover_image_keyed(bytes: &[u8], key_hex: &str) -> Result<String, JsValue> {
    let key = optional_key(key_hex)?;
    let capsule = if bytes.starts_with(b"\x89PNG") {
        decode_png_with_key(bytes, key.as_ref())
    } else {
        std::str::from_utf8(bytes)
            .map_err(|_| "Not a source-carrying PNG or SVG".to_string())
            .and_then(|svg| decode_svg_with_key(svg, key.as_ref()))
    }
    .map_err(|e| JsValue::from_str(&e))?;
    serde_json::to_string(&capsule).map_err(|e| JsValue::from_str(&e.to_string()))
}
#[wasm_bindgen]
pub fn recover_names_keyed(svg: &str, key_hex: &str) -> Result<String, JsValue> {
    let key = optional_key(key_hex)?;
    let names = decode_svg_names_with_key(svg, key.as_ref()).map_err(|e| JsValue::from_str(&e))?;
    serde_json::to_string(&names).map_err(|e| JsValue::from_str(&e.to_string()))
}
