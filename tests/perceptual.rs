use code_is_magic::{
    decode_image_with_key, encode_perceptual_jpeg_with_key, ArchiveEntry, Capsule, MagicKey,
    Snapshot, DEFAULT_PERCEPTUAL_QUALITY,
};

fn capsule() -> Capsule {
    Capsule {
        version: 1,
        snapshot: Snapshot {
            version: 1,
            title: "perceptual-test".into(),
            commit: Some("0123456789abcdef0123456789abcdef01234567".into()),
            entries: vec![ArchiveEntry {
                path: "src/main.rs".into(),
                mode: "100644".into(),
                data: "Zm4gbWFpbigpIHt9Cg==".into(),
                sha: None,
            }],
        },
        svg: "<svg xmlns='http://www.w3.org/2000/svg' width='320' height='240' viewBox='0 0 320 240'><rect width='320' height='240' fill='#090d20'/><circle cx='160' cy='120' r='86' fill='none' stroke='#f5dfaa' stroke-width='2'/><path d='M90 120L160 54L230 120L160 186Z' fill='none' stroke='#d8b86a' stroke-width='2'/></svg>".into(),
    }
}

#[test]
fn perceptual_jpeg_is_deterministic_and_source_exact() {
    let capsule = capsule();
    let first =
        encode_perceptual_jpeg_with_key(&capsule, None, DEFAULT_PERCEPTUAL_QUALITY).unwrap();
    let second =
        encode_perceptual_jpeg_with_key(&capsule, None, DEFAULT_PERCEPTUAL_QUALITY).unwrap();
    assert_eq!(first, second);
    assert!(first.starts_with(&[0xff, 0xd8]));
    assert!(first.windows(8).any(|w| w == b"CIMJPG01"));
    let recovered = decode_image_with_key(&first, None).unwrap();
    assert_eq!(recovered.snapshot, capsule.snapshot);
    assert!(recovered.svg.is_empty());
}

#[test]
fn perceptual_jpeg_keyed_source_requires_the_original_key() {
    let capsule = capsule();
    let key = MagicKey::from_hex(&"31".repeat(32)).unwrap();
    let wrong = MagicKey::from_hex(&"32".repeat(32)).unwrap();
    let bytes = encode_perceptual_jpeg_with_key(&capsule, Some(&key), 96).unwrap();
    assert!(decode_image_with_key(&bytes, None).is_err());
    assert!(decode_image_with_key(&bytes, Some(&wrong)).is_err());
    assert_eq!(
        decode_image_with_key(&bytes, Some(&key)).unwrap().snapshot,
        capsule.snapshot
    );
}

#[test]
fn perceptual_profile_rejects_quality_that_can_be_visibly_lossy() {
    let capsule = capsule();
    assert!(encode_perceptual_jpeg_with_key(&capsule, None, 94).is_err());
}
