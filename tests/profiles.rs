use code_is_magic::{analyze, language_profiles, profile_for_path, supported, SourceFile};
use std::collections::BTreeSet;

#[test]
fn bundled_profiles_cover_nine_languages_with_unambiguous_extensions() {
    let profiles = language_profiles();
    assert_eq!(profiles.len(), 9);
    let mut extensions = BTreeSet::new();
    for profile in profiles {
        assert!(!profile.collar.is_empty() && !profile.sigil.is_empty());
        for extension in &profile.extensions {
            assert!(extensions.insert(extension));
            let path = format!("source.{extension}");
            assert!(supported(&path));
            assert_eq!(profile_for_path(&path).unwrap().id, profile.id);
        }
    }
    assert_eq!(extensions.len(), 16);
    assert!(!supported("unrecognized.txt"));
}

#[test]
fn dialect_groups_resolve_ecmascript_and_c_family_calls_without_crossing_other_languages() {
    let files = [
        ("caller.ts", "function caller() { helper(); }"),
        ("helper.js", "function helper() { return; }"),
        ("caller.cpp", "void cpp_call() { c_helper(); }"),
        ("helper.c", "void c_helper() { return; }"),
        ("other.rs", "fn helper() { return; }"),
    ]
    .map(|(path, content)| SourceFile {
        path: path.into(),
        content: content.into(),
    });
    let analysis = analyze(&files);
    let targets: BTreeSet<_> = analysis
        .edges
        .iter()
        .filter(|edge| edge.kind == "calls")
        .map(|edge| {
            (
                analysis.entities[edge.from].name.as_str(),
                analysis.entities[edge.to].path.as_str(),
            )
        })
        .collect();
    assert_eq!(
        targets,
        BTreeSet::from([("caller", "helper.js"), ("cpp_call", "helper.c")])
    );
    assert_eq!(analysis.unresolved_calls, 0);
}
