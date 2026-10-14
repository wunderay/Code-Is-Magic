mod analyze;
mod archive;
mod render;
mod sigil;
pub use analyze::{analyze, supported, Analysis, Edge, Entity, SourceFile};
pub use archive::{
    decode_png, decode_svg, embed_svg, encode_png, snapshot_zip, ArchiveEntry, Capsule, Snapshot,
};
pub use render::{render, render_with_wingdings, Diagram, Placement};
pub use sigil::{decode_name, decode_svg_names, encode_name, DecodedName, NameSigil};
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
    let files: Vec<SourceFile> =
        serde_json::from_str(files_json).map_err(|e| JsValue::from_str(&e.to_string()))?;
    let analysis = analyze(&files);
    serde_json::to_string(&render_with_wingdings(
        &analysis,
        title,
        min_lines.max(1),
        palette,
        wingdings,
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
