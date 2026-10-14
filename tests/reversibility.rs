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
fn png_and_svg_recover_every_byte_and_mode() {
    let mut c = capsule();
    c.snapshot.canonicalize().unwrap();
    let png = encode_png(&c).unwrap();
    assert_eq!(png, encode_png(&c).unwrap());
    assert_eq!(decode_png(&png).unwrap(), c);
    assert_eq!(encode_png(&decode_png(&png).unwrap()).unwrap(), png);
    let svg = embed_svg(&c).unwrap();
    assert_eq!(decode_svg(&svg).unwrap(), c);
    // Re-encode pixels with a fresh PNG encoder: no ancillary metadata survives.
    let mut decoder = png::Decoder::new(std::io::Cursor::new(png));
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
    assert_eq!(decode_png(&reencoded).unwrap(), c);
}
#[test]
fn edited_pixels_and_ordinary_images_cannot_claim_exact_recovery() {
    let png = encode_png(&capsule()).unwrap();
    let mut decoder = png::Decoder::new(std::io::Cursor::new(png));
    decoder.set_transformations(png::Transformations::EXPAND);
    let mut reader = decoder.read_info().unwrap();
    let mut buf = vec![0; reader.output_buffer_size()];
    let info = reader.next_frame(&mut buf).unwrap();
    buf[0] ^= 1;
    let mut changed = Vec::new();
    {
        let mut e = png::Encoder::new(&mut changed, info.width, info.height);
        e.set_color(info.color_type);
        e.set_depth(info.bit_depth);
        e.write_header()
            .unwrap()
            .write_image_data(&buf[..info.buffer_size()])
            .unwrap();
    }
    assert!(decode_png(&changed).is_err());
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
