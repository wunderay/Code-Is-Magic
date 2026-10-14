mod analyze;
mod render;
pub use analyze::{analyze, supported, Analysis, Edge, Entity, SourceFile};
pub use render::{render, render_with_wingdings, Diagram, Placement};
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
