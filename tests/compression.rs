use base64::Engine;
use code_is_magic::*;
use std::io::Write;

fn pixels(bytes: &[u8]) -> (u32, u32, Vec<u8>) {
    let mut decoder = png::Decoder::new(bytes);
    decoder.set_transformations(png::Transformations::EXPAND);
    let mut reader = decoder.read_info().unwrap();
    let mut data = vec![0; reader.output_buffer_size()];
    let frame = reader.next_frame(&mut data).unwrap();
    let rgba = data[..frame.buffer_size()]
        .chunks_exact(frame.color_type.samples())
        .flat_map(|p| match frame.color_type {
            png::ColorType::Grayscale => [p[0], p[0], p[0], 255],
            png::ColorType::GrayscaleAlpha => [p[0], p[0], p[0], p[1]],
            png::ColorType::Rgb => [p[0], p[1], p[2], 255],
            _ => [p[0], p[1], p[2], p[3]],
        })
        .collect();
    (frame.width, frame.height, rgba)
}

#[test]
fn png_optimization_preserves_all_pixels_including_alpha_and_color_profile() {
    for (colors, gray, transparent) in [
        (2, false, false),
        (4, false, true),
        (16, false, false),
        (200, false, true),
        (300, false, false),
        (40, true, false),
        (40, true, true),
    ] {
        let width = 37;
        let height = 29;
        let data: Vec<u8> = (0..width * height)
            .flat_map(|i| {
                let n = i % colors;
                if gray {
                    [
                        n as u8,
                        n as u8,
                        n as u8,
                        if transparent { (n * 7) as u8 } else { 255 },
                    ]
                } else {
                    [
                        n as u8,
                        (n * 53) as u8,
                        (n / 256 * 23 + 47) as u8,
                        if transparent { (n * 19) as u8 } else { 255 },
                    ]
                }
            })
            .collect();
        let mut original = Vec::new();
        let mut encoder = png::Encoder::new(&mut original, width, height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_source_srgb(png::SrgbRenderingIntent::Perceptual);
        encoder.set_compression(png::Compression::Fast);
        encoder
            .write_header()
            .unwrap()
            .write_image_data(&data)
            .unwrap();
        let optimized = optimize_png(&original).unwrap();
        assert!(optimized.len() <= original.len());
        assert_eq!(pixels(&optimized), pixels(&original));
        assert_eq!(optimized, optimize_png(&original).unwrap());
        let reader = png::Decoder::new(optimized.as_slice()).read_info().unwrap();
        assert_eq!(
            reader.info().srgb,
            Some(png::SrgbRenderingIntent::Perceptual)
        );
    }
}

fn capsule() -> Capsule {
    let source = "fn light() { sparkle(); }\nfn sparkle() { let rune = 42; }\n";
    let snapshot = Snapshot {
        version: 1,
        title: "compression fixture".into(),
        commit: None,
        entries: vec![ArchiveEntry {
            path: "spell.rs".into(),
            mode: "100755".into(),
            data: base64::engine::general_purpose::STANDARD.encode(source),
            sha: None,
        }],
    };
    Capsule {
        version: 1,
        svg: render(&analyze(&snapshot.source_files()), "test", 1, "gold").svg,
        snapshot,
    }
}
fn raster(svg: &str) -> Vec<u8> {
    let tree = resvg::usvg::Tree::from_str(svg, &resvg::usvg::Options::default()).unwrap();
    let mut image = resvg::tiny_skia::Pixmap::new(960, 960).unwrap();
    resvg::render(
        &tree,
        resvg::tiny_skia::Transform::from_scale(
            960. / tree.size().width(),
            960. / tree.size().height(),
        ),
        &mut image.as_mut(),
    );
    image.take()
}

#[test]
fn shared_outlines_preserve_rendering_and_legacy_compaction_is_idempotent() {
    let c = capsule();
    let doc = roxmltree::Document::parse(&c.svg).unwrap();
    let mut inline = c.svg.clone();
    let replacements: Vec<_> = doc
        .descendants()
        .filter(|n| n.has_tag_name("use"))
        .map(|n| {
            let href = n.attribute("href").unwrap();
            let target = doc
                .descendants()
                .find(|t| t.attribute("id") == Some(&href[1..]))
                .unwrap();
            let mut tag = c.svg[n.range()].to_string();
            let attr = n.attribute_node("href").unwrap().range();
            tag.replace_range(
                attr.start - n.range().start..attr.end - n.range().start,
                &format!("d='{}'", target.attribute("d").unwrap()),
            );
            tag.replace_range(1..4, "path");
            (n.range(), tag)
        })
        .collect();
    for (range, tag) in replacements.into_iter().rev() {
        inline.replace_range(range, &tag);
    }
    assert_eq!(raster(&c.svg), raster(&inline));
    let compact = compact_svg(&inline).unwrap();
    assert!(compact.len() < inline.len());
    assert_eq!(raster(&compact), raster(&inline));
    assert_eq!(compact_svg(&compact).unwrap(), compact);
    assert_eq!(
        decode_svg_names(&compact).unwrap(),
        decode_svg_names(&inline).unwrap()
    );
}

#[test]
fn old_zlib_svg_new_svgz_and_canonical_png_restore_exact_source() {
    let c = capsule();
    let mut z = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::new(6));
    z.write_all(&serde_json::to_vec(&c).unwrap()).unwrap();
    let old_payload = base64::engine::general_purpose::STANDARD.encode(z.finish().unwrap());
    let old = c.svg.replace(
        "</svg>",
        &format!("<metadata id='source-capsule-v1'>{old_payload}</metadata></svg>"),
    );
    assert_eq!(decode_svg(&old).unwrap(), c);
    for key in [None, Some(MagicKey::from_hex(&"01".repeat(32)).unwrap())] {
        let svg = embed_svg_with_key(&c, key.as_ref()).unwrap();
        let svgz = encode_svgz_with_key(&c, key.as_ref()).unwrap();
        assert!(svgz.len() < svg.len());
        assert_eq!(decode_image_with_key(&svgz, key.as_ref()).unwrap(), c);
        assert_eq!(svgz, encode_svgz_with_key(&c, key.as_ref()).unwrap());
        assert!(decode_image_with_key(&svgz[..svgz.len() - 8], key.as_ref()).is_err());
        assert_eq!(raster(&svg), raster(&c.svg));
        let png = encode_png_with_key(&c, key.as_ref()).unwrap();
        let recovered = decode_png_with_key(&png, key.as_ref()).unwrap();
        assert_eq!(recovered.snapshot, c.snapshot);
        assert_eq!(raster(&recovered.svg), raster(&c.svg));
        assert_eq!(
            decode_svg_names_with_key(&recovered.svg, key.as_ref()).unwrap(),
            decode_svg_names_with_key(&c.svg, key.as_ref()).unwrap()
        );
    }
}
