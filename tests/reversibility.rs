use base64::Engine;
use code_is_magic::*;
fn entry(path: &str, mode: &str, bytes: &[u8]) -> ArchiveEntry {
    ArchiveEntry {
        path: path.into(),
        mode: mode.into(),
        data: base64::engine::general_purpose::STANDARD.encode(bytes),
        sha: None,
    }
}
fn capsule() -> Capsule {
    let snapshot = Snapshot {
        version: 1,
        title: "not shown".into(),
        commit: None,
        entries: vec![
            entry(
                "code.rs",
                "100644",
                b"\xef\xbb\xbffn spell() {\r\n  return;\r\n}\r\n",
            ),
            entry("binary.dat", "100644", &[0, 255, 128, 0, 1, 2]),
            entry("run.sh", "100755", b"#!/bin/sh\n"),
            entry("link", "120000", b"code.rs"),
            ArchiveEntry {
                path: "module".into(),
                mode: "160000".into(),
                data: String::new(),
                sha: Some("a".repeat(40)),
            },
        ],
    };
    let svg = render(&analyze(&snapshot.source_files()), "ignored", 12, "gold").svg;
    Capsule {
        version: 1,
        snapshot,
        svg,
    }
}
#[test]
fn name_contours_are_exact_injective_and_deterministic() {
    let mut shapes = std::collections::BTreeSet::new();
    for name in [
        "",
        "a",
        "A",
        "cast",
        "cast_",
        "Book::cast",
        "Δράκος",
        "魔法",
        "ᚠᛟ",
        "a\0b",
        "é",
        "e\u{301}",
    ] {
        let sigil = encode_name(name);
        assert_eq!(decode_name(&sigil).unwrap(), name);
        assert_eq!(sigil, encode_name(name));
        assert!(shapes.insert(sigil.svg_points()));
    }
    for i in 0..1000 {
        let name = format!("Spell::{i}::魔法");
        assert_eq!(decode_name(&encode_name(&name)).unwrap(), name);
    }
}
#[test]
fn svg_geometry_recovers_names_without_text_or_name_metadata() {
    let files = [SourceFile {
        path: "secret-filename.rs".into(),
        content: "fn cast() {} fn shine() {}".into(),
    }];
    let d = render(&analyze(&files), "secret title", 12, "gold");
    assert!(!d.svg.contains("<text") && !d.svg.contains("<title"));
    for name in ["secret-filename", "secret title", "cast", "shine"] {
        assert!(!d.svg.contains(name));
    }
    let recovered = decode_svg_names(&d.svg).unwrap();
    assert_eq!(
        recovered
            .iter()
            .map(|n| n.name.as_str())
            .collect::<Vec<_>>(),
        vec!["cast", "shine"]
    );
}
#[test]
fn same_qualified_name_has_same_contour_after_file_or_body_changes() {
    let a = encode_name("Book::cast");
    let b = encode_name("Book::cast");
    assert_eq!(a, b);
    let mut damaged = a.clone();
    damaged.points[8][0] += 0.01;
    assert!(decode_name(&damaged).is_err());
}
#[test]
fn canonical_png_and_legacy_svg_recover_every_source_byte_and_mode() {
    let mut c = capsule();
    c.snapshot.canonicalize().unwrap();
    let png = encode_png(&c).unwrap();
    assert_eq!(png, encode_png(&c).unwrap());
    let recovered = decode_png(&png).unwrap();
    assert_eq!(recovered.snapshot, c.snapshot);
    assert_eq!(decode_svg_names(&recovered.svg).unwrap(), decode_svg_names(&c.svg).unwrap());
    // Re-encoding a recovered canonical carrier is deterministic after SVG compaction.
    assert_eq!(encode_png(&recovered).unwrap(), encode_png(&recovered).unwrap());
    // The older source-carrying SVG format remains readable for backward compatibility.
    let svg = embed_svg(&c).unwrap();
    assert_eq!(decode_svg(&svg).unwrap(), c);
}
#[test]
fn canonical_png_uses_a_separate_source_chunk_not_visible_pixel_bits() {
    let c = capsule();
    let png = encode_png(&c).unwrap();
    assert!(png.windows(4).any(|w| w == b"ciMg"));
    // A generic pixel-only re-encode deliberately strips private source data.
    let mut decoder = png::Decoder::new(std::io::Cursor::new(&png));
    decoder.set_transformations(png::Transformations::EXPAND);
    let mut reader = decoder.read_info().unwrap();
    let mut buf = vec![0; reader.output_buffer_size()];
    let info = reader.next_frame(&mut buf).unwrap();
    let mut reencoded = Vec::new();
    {
        let mut e = png::Encoder::new(&mut reencoded, info.width, info.height);
        e.set_color(info.color_type);
        e.set_depth(info.bit_depth);
        e.write_header()
            .unwrap()
            .write_image_data(&buf[..info.buffer_size()])
            .unwrap();
    }
    assert!(decode_png(&reencoded).is_err());
    // Code Is Magic's own optimizer keeps the source chunk intact.
    let optimized = optimize_png(&png).unwrap();
    assert_eq!(decode_png(&optimized).unwrap().snapshot, c.snapshot);
}
#[test]
fn visually_imperceptible_raster_is_bounded_to_one_code_value_per_rgb_channel() {
    let c = capsule();
    let png = encode_png(&c).unwrap();
    let mut decoder = png::Decoder::new(std::io::Cursor::new(&png));
    decoder.set_transformations(png::Transformations::EXPAND);
    let mut reader = decoder.read_info().unwrap();
    let width = reader.info().width;
    let height = reader.info().height;
    let mut encoded = vec![0; reader.output_buffer_size()];
    let frame = reader.next_frame(&mut encoded).unwrap();
    let channels = frame.color_type.samples();
    let tree = resvg::usvg::Tree::from_str(&c.svg, &resvg::usvg::Options::default()).unwrap();
    let mut reference = resvg::tiny_skia::Pixmap::new(width, height).unwrap();
    resvg::render(
        &tree,
        resvg::tiny_skia::Transform::from_scale(
            width as f32 / tree.size().width(),
            height as f32 / tree.size().height(),
        ),
        &mut reference.as_mut(),
    );
    for (i, original) in reference.data().as_chunks::<4>().0.iter().enumerate() {
        let output = &encoded[i * channels..i * channels + channels];
        let rgb = if channels >= 3 {
            [output[0], output[1], output[2]]
        } else {
            [output[0]; 3]
        };
        for channel in 0..3 {
            assert!((original[channel] as i16 - rgb[channel] as i16).abs() <= 1);
        }
    }
}
#[test]
fn ordinary_images_cannot_claim_exact_recovery() {
    assert!(decode_png(b"not an image").is_err());
}
#[test]
fn unsafe_and_duplicate_paths_are_rejected() {
    let mut c = capsule();
    c.snapshot.entries[0].path = "../escape".into();
    assert!(encode_png(&c).is_err());
    let mut c = capsule();
    c.snapshot.entries.push(c.snapshot.entries[0].clone());
    assert!(snapshot_zip(&c.snapshot).is_err());
}
#[test]
fn zip_preserves_symlink_and_executable_attributes() {
    let c = capsule();
    let zip = snapshot_zip(&c.snapshot).unwrap();
    assert!(zip.starts_with(b"PK\x03\x04"));
    assert_eq!(zip, snapshot_zip(&c.snapshot).unwrap());
}
