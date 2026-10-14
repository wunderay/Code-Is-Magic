mod analyze;
mod archive;
mod compression;
mod crypto;
pub use compression::{compact_svg, optimize_png};
pub use crypto::MagicKey;
mod inscription;
mod perceptual;
mod profiles;
mod render;
mod sigil;
pub use analyze::{analyze, supported, Analysis, Edge, Entity, SourceFile};
pub use archive::{
    decode_png, decode_png_with_key, decode_svg, decode_svg_with_key, embed_svg,
    embed_svg_with_key, encode_png, encode_png_with_key, encode_svgz_with_key, snapshot_zip,
    ArchiveEntry, Capsule, Snapshot,
};
pub use perceptual::{
    decode_perceptual_jpeg, decode_perceptual_jpeg_with_key, encode_perceptual_jpeg,
    encode_perceptual_jpeg_with_key, DEFAULT_PERCEPTUAL_QUALITY,
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

/// Recover either an exact PNG/SVG/SVGZ carrier or a perceptual JPEG carrier.
pub fn decode_image_with_key(bytes: &[u8], key: Option<&MagicKey>) -> Result<Capsule, String> {
    if bytes.starts_with(&[0xff, 0xd8]) {
        perceptual::decode_perceptual_jpeg_with_key(bytes, key)
    } else {
        archive::decode_image_with_key(bytes, key)
    }
}

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
#[wasm_bindgen(js_name = optimize_png)]
pub fn optimize_png_browser(bytes: &[u8]) -> Result<Vec<u8>, JsValue> {
    optimize_png(bytes).map_err(|e| JsValue::from_str(&e))
}
#[wasm_bindgen]
pub fn pack_svg(capsule_json: &str) -> Result<String, JsValue> {
    let capsule: Capsule =
        serde_json::from_str(capsule_json).map_err(|e| JsValue::from_str(&e.to_string()))?;
    embed_svg(&capsule).map_err(|e| JsValue::from_str(&e))
}
#[wasm_bindgen]
pub fn recover_image(bytes: &[u8]) -> Result<String, JsValue> {
    let capsule = decode_image_with_key(bytes, None).map_err(|e| JsValue::from_str(&e))?;
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
pub fn pack_jpeg_keyed(
    capsule_json: &str,
    key_hex: &str,
    quality: u8,
) -> Result<Vec<u8>, JsValue> {
    let key = optional_key(key_hex)?;
    let capsule: Capsule =
        serde_json::from_str(capsule_json).map_err(|e| JsValue::from_str(&e.to_string()))?;
    encode_perceptual_jpeg_with_key(&capsule, key.as_ref(), quality)
        .map_err(|e| JsValue::from_str(&e))
}
#[wasm_bindgen]
pub fn pack_svg_keyed(capsule_json: &str, key_hex: &str) -> Result<String, JsValue> {
    let key = optional_key(key_hex)?;
    let mut capsule: Capsule =
        serde_json::from_str(capsule_json).map_err(|e| JsValue::from_str(&e.to_string()))?;
    capsule.svg = compact_svg(&capsule.svg).map_err(|e| JsValue::from_str(&e))?;
    embed_svg_with_key(&capsule, key.as_ref()).map_err(|e| JsValue::from_str(&e))
}
#[wasm_bindgen]
pub fn recover_image_keyed(bytes: &[u8], key_hex: &str) -> Result<String, JsValue> {
    let key = optional_key(key_hex)?;
    let capsule = decode_image_with_key(bytes, key.as_ref()).map_err(|e| JsValue::from_str(&e))?;
    serde_json::to_string(&capsule).map_err(|e| JsValue::from_str(&e.to_string()))
}
#[wasm_bindgen]
pub fn recover_names_keyed(svg: &str, key_hex: &str) -> Result<String, JsValue> {
    let key = optional_key(key_hex)?;
    let names = decode_svg_names_with_key(svg, key.as_ref()).map_err(|e| JsValue::from_str(&e))?;
    serde_json::to_string(&names).map_err(|e| JsValue::from_str(&e.to_string()))
}

#[wasm_bindgen]
pub fn pack_svgz_keyed(capsule_json: &str, key_hex: &str) -> Result<Vec<u8>, JsValue> {
    let key = optional_key(key_hex)?;
    let mut capsule: Capsule =
        serde_json::from_str(capsule_json).map_err(|e| JsValue::from_str(&e.to_string()))?;
    capsule.svg = compact_svg(&capsule.svg).map_err(|e| JsValue::from_str(&e))?;
    encode_svgz_with_key(&capsule, key.as_ref()).map_err(|e| JsValue::from_str(&e))
}
