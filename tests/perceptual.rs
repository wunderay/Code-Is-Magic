use code_is_magic::{analyze, perceptual_svg, render_with_options, SourceFile};

fn raster(svg: &str, width: u32, height: u32) -> Vec<u8> {
    let tree = resvg::usvg::Tree::from_str(svg, &resvg::usvg::Options::default()).unwrap();
    let mut image = resvg::tiny_skia::Pixmap::new(width, height).unwrap();
    resvg::render(
        &tree,
        resvg::tiny_skia::Transform::from_scale(
            width as f32 / tree.size().width(),
            height as f32 / tree.size().height(),
        ),
        &mut image.as_mut(),
    );
    image.take()
}

fn representative_source() -> SourceFile {
    let mut content = String::from(
        "pub struct Grimoire { pub charge: usize }\n\
         pub enum School { Solar, Lunar, Astral, Runic }\n\
         impl Grimoire {\n\
         pub fn invoke(&mut self) { spell_0(); spell_3(); spell_7(); self.charge += 1; }\n\
         pub fn recover(&self) -> usize { self.charge }\n\
         }\n",
    );
    for i in 0..24 {
        let next = (i + 1) % 24;
        let cross = (i + 7) % 24;
        content.push_str(&format!("pub fn spell_{i}() {{\n"));
        for j in 0..(3 + i % 11) {
            content.push_str(&format!("    let rune_{j} = {i}usize.wrapping_mul({j});\n"));
        }
        content.push_str(&format!("    if {i} % 3 == 0 {{ spell_{next}(); }}\n"));
        content.push_str(&format!("    if {i} % 5 == 0 {{ spell_{cross}(); }}\n"));
        content.push_str("}\n");
    }
    SourceFile {
        path: "src/grimoire.rs".into(),
        content,
    }
}

#[test]
fn canonical_geometry_stays_below_a_conservative_visual_error_floor() {
    let analysis = analyze(&[representative_source()]);
    let diagram = render_with_options(&analysis, "perceptual-regression", 12, "gold", true, true);
    let perceptual = perceptual_svg(&diagram.svg).unwrap();
    assert!(
        perceptual.len() <= diagram.svg.len(),
        "perceptual geometry unexpectedly expanded the raw SVG"
    );

    let original = raster(&diagram.svg, diagram.width, diagram.height);
    let compressed = raster(&perceptual, diagram.width, diagram.height);
    assert_eq!(original.len(), compressed.len());

    let mut squared_error = 0.0_f64;
    let mut absolute_error = 0.0_f64;
    for (&a, &b) in original.iter().zip(&compressed) {
        let delta = a as f64 - b as f64;
        squared_error += delta * delta;
        absolute_error += delta.abs();
    }
    let samples = original.len() as f64;
    let mse = squared_error / samples;
    let mae = absolute_error / samples;
    let psnr = if mse == 0.0 {
        f64::INFINITY
    } else {
        10.0 * ((255.0 * 255.0) / mse).log10()
    };

    // The production reference drawing measured ~51 dB / ~0.08 MAE. These gates
    // intentionally leave headroom for harmless renderer evolution while rejecting
    // precision changes large enough to become visible on thin glyphs and channels.
    assert!(psnr >= 45.0, "perceptual PSNR regressed to {psnr:.2} dB");
    assert!(mae <= 0.35, "perceptual MAE regressed to {mae:.4}");
}
