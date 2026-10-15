use base64::Engine;
use code_is_magic::*;
fn key(byte: u8) -> MagicKey {
    MagicKey::from_hex(&format!("{byte:02x}").repeat(32)).unwrap()
}
fn capsule(key: Option<&MagicKey>) -> Capsule {
    let mut snapshot = Snapshot {
        version: 1,
        title: "private title".into(),
        commit: None,
        entries: vec![
            ArchiveEntry {
                path: "secret.rs".into(),
                mode: "100755".into(),
                sha: None,
                data: base64::engine::general_purpose::STANDARD.encode(
                    b"\xef\xbb\xbffn alpha() {\r\n beta();\r\n}\r\nfn beta() { return; }\r\n",
                ),
            },
            ArchiveEntry {
                path: "binary".into(),
                mode: "100644".into(),
                sha: None,
                data: base64::engine::general_purpose::STANDARD.encode([0, 255, 128, 0]),
            },
            ArchiveEntry {
                path: "link".into(),
                mode: "120000".into(),
                sha: None,
                data: base64::engine::general_purpose::STANDARD.encode(b"secret.rs"),
            },
            ArchiveEntry {
                path: "module".into(),
                mode: "160000".into(),
                sha: Some("a".repeat(40)),
                data: "".into(),
            },
        ],
    };
    snapshot.canonicalize().unwrap();
    let svg = render_with_key(
        &analyze(&snapshot.source_files()),
        "ignored",
        1,
        "gold",
        true,
        true,
        key,
    )
    .unwrap()
    .svg;
    Capsule {
        version: 1,
        snapshot,
        svg,
    }
}
#[test]
fn keyed_names_are_injective_reversible_and_require_the_key() {
    let (a, b) = (key(1), key(2));
    let mut shapes = std::collections::BTreeSet::new();
    for i in 0..200 {
        let name = format!("魔法::Δ::{i}\0é");
        let sigil = encode_name_with_key(&name, Some(&a)).unwrap();
        assert!(shapes.insert(sigil.svg_points()));
        assert_eq!(sigil, encode_name_with_key(&name, Some(&a)).unwrap());
        assert_ne!(sigil, encode_name_with_key(&name, Some(&b)).unwrap());
        assert_eq!(decode_name_with_key(&sigil, Some(&a)).unwrap(), name);
        assert!(decode_name(&sigil).is_err());
        assert!(decode_name_with_key(&sigil, Some(&b)).is_err());
    }
    for name in ["", "A", "a", "é", "e\u{301}"] {
        let sigil = encode_name_with_key(name, Some(&a)).unwrap();
        assert_eq!(decode_name_with_key(&sigil, Some(&a)).unwrap(), name);
    }
}
#[test]
fn keyed_art_changes_centers_glyphs_and_channels_but_preserves_size_and_graph() {
    let (a, b) = (key(1), key(2));
    let files = capsule(None).snapshot.source_files();
    let analysis = analyze(&files);
    let render = |key| render_with_key(&analysis, "", 1, "gold", false, true, key).unwrap();
    let (plain, one, two) = (render(None), render(Some(&a)), render(Some(&b)));
    assert_eq!(one.svg, render(Some(&a)).svg);
    assert_ne!(plain.svg, one.svg);
    assert_ne!(one.svg, two.svg);
    for p in &plain.placements {
        assert_eq!(
            p.radius,
            one.placements.iter().find(|q| q.id == p.id).unwrap().radius
        );
    }
    assert_eq!(
        serde_json::to_value(&plain.analysis).unwrap(),
        serde_json::to_value(&one.analysis).unwrap()
    );
    for (start, end) in [
        ("<g data-center=", "</g>"),
        ("<g data-channel=", "<g id='entity-"),
        ("<g fill='#", "</svg>"),
    ] {
        let section = |svg: &str| {
            svg.split_once(start)
                .unwrap()
                .1
                .split_once(end)
                .unwrap()
                .0
                .to_owned()
        };
        assert_ne!(section(&one.svg), section(&two.svg));
    }
    assert!(!one.svg.contains("<text"));
    assert!(!one.svg.contains("marker-end"));
    assert!(decode_svg_names(&one.svg).is_err());
    assert_eq!(
        decode_svg_names_with_key(&one.svg, Some(&a)).unwrap().len(),
        2
    );
}
#[test]
fn encrypted_images_restore_exact_snapshot_and_reject_wrong_keys_and_tampering() {
    let (a, b) = (key(1), key(2));
    let c = capsule(Some(&a));
    let png = encode_png_with_key(&c, Some(&a)).unwrap();
    assert_eq!(png, encode_png_with_key(&c, Some(&a)).unwrap());
    assert!(decode_png(&png).is_err());
    assert!(decode_png_with_key(&png, Some(&b)).is_err());
    let recovered = decode_png_with_key(&png, Some(&a)).unwrap();
    assert_eq!(recovered.snapshot, c.snapshot);
    assert_eq!(
        decode_svg_names_with_key(&recovered.svg, Some(&a)).unwrap(),
        decode_svg_names_with_key(&c.svg, Some(&a)).unwrap()
    );
    assert_eq!(
        snapshot_zip(&recovered.snapshot).unwrap(),
        snapshot_zip(&c.snapshot).unwrap()
    );
    assert_eq!(
        encode_png_with_key(&recovered, Some(&a)).unwrap(),
        encode_png_with_key(&recovered, Some(&a)).unwrap()
    );
    let svg = embed_svg_with_key(&c, Some(&a)).unwrap();
    assert_eq!(decode_svg_with_key(&svg, Some(&a)).unwrap(), c);
    assert!(decode_svg(&svg).is_err());
    assert!(decode_svg_with_key(&svg, Some(&b)).is_err());
    assert!(!svg.contains("private title"));
    assert!(!svg.contains("secret.rs"));
    let changed = svg.replacen("#0c1112", "#0d1112", 1);
    assert!(decode_svg_with_key(&changed, Some(&a)).is_err());
    // Mutate ciphertext and re-encode valid Base64: authentication must catch it.
    let marker = "<metadata id='source-capsule-v1'>";
    let data = svg
        .split_once(marker)
        .unwrap()
        .1
        .split_once("</metadata>")
        .unwrap()
        .0;
    let mut bytes = base64::engine::general_purpose::STANDARD
        .decode(data)
        .unwrap();
    bytes[30] ^= 1;
    let changed = svg.replace(
        data,
        &base64::engine::general_purpose::STANDARD.encode(bytes),
    );
    assert!(decode_svg_with_key(&changed, Some(&a)).is_err());
    assert!(encode_png(&c).is_err());
    assert!(embed_svg(&c).is_err());
    assert!(encode_png_with_key(&c, Some(&b)).is_err());
    assert!(embed_svg_with_key(&c, Some(&b)).is_err());
}
#[test]
fn legacy_images_work_and_invalid_keys_never_echo_secrets() {
    let a = key(1);
    let c = capsule(None);
    let legacy = encode_png_legacy(&c).unwrap();
    assert_eq!(
        decode_png_with_key(&legacy, Some(&a)).unwrap(),
        c
    );
    assert_eq!(
        decode_svg_with_key(&embed_svg(&c).unwrap(), Some(&a)).unwrap(),
        c
    );
    for invalid in ["", "my-secret-password", &"é".repeat(32), &"g".repeat(64)] {
        let error = MagicKey::from_hex(invalid).err().unwrap();
        if !invalid.is_empty() {
            assert!(!error.contains(invalid));
        }
    }
}
