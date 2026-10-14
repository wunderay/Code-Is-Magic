use code_is_magic::{analyze, render, SourceFile};
fn file(path: &str, content: &str) -> SourceFile {
    SourceFile {
        path: path.into(),
        content: content.into(),
    }
}
#[test]
fn ignores_comments_and_strings_and_links_real_calls() {
    let a = analyze(&[file(
        "spell.rs",
        r#"// fn counterfeit() { evil(); }
fn caller() { let text = "fn illusion() { evil(); }"; callee(); }
fn callee() { caller(); }"#,
    )]);
    assert_eq!(a.entities.len(), 2);
    assert_eq!(a.edges.iter().filter(|e| e.kind == "calls").count(), 2);
}
#[test]
fn rust_impl_ownership_self_calls_and_recursion() {
    let a=analyze(&[file("spell.rs","struct Book { ink: u32 } impl Book { fn open(&self) { self.close(); } fn close(&self) { self.close(); } }")]);
    let book = a.entities.iter().find(|e| e.kind == "struct").unwrap();
    let open = a.entities.iter().find(|e| e.name == "open").unwrap();
    let close = a.entities.iter().find(|e| e.name == "close").unwrap();
    assert_eq!(open.owner, Some(book.id));
    assert!(a
        .edges
        .iter()
        .any(|e| e.from == open.id && e.to == close.id && e.kind == "calls"));
    assert!(a
        .edges
        .iter()
        .any(|e| e.from == close.id && e.to == close.id));
}
#[test]
fn python_indentation_nested_functions_and_triple_strings() {
    let a=analyze(&[file("moon.py","class Moon:\n    async def rise(self):\n        \"\"\"def fake():\n            evil()\"\"\"\n        def nested():\n            other()\n        self.set()\n        nested()\n    def set(self):\n        return 1\n")]);
    assert_eq!(
        a.entities.iter().filter(|e| e.kind == "function").count(),
        3
    );
    let rise = a.entities.iter().find(|e| e.name == "rise").unwrap();
    assert!(!rise.calls.contains(&"other".into()));
    assert!(rise.calls.contains(&"self::set".into()));
}
#[test]
fn handles_language_signatures_types_methods_and_arrow_functions() {
    let cases=[("spell.ts","interface Spell { power: number; } const cast = (power: number) => { shine(power); }; function shine(n: number) { return n; }","TypeScript"),
 ("spell.js","const altar = { power: 3 }; function cast() { shine(); } function shine() {}","JavaScript"),
 ("spell.go","type Book struct { Pages int }; func cast(n int) { shine(n) }; func shine(n int) { println(n) }","Go"),
 ("spell.java","class Book { void cast() { shine(); } void shine() { return; } }","Java"),
 ("spell.cs","class Book { void Cast() { Shine(); } void Shine() { return; } }","C#"),
 ("spell.c","struct Book { int pages; }; void cast() { shine(); } void shine() { return; }","C"),
 ("spell.cpp","class Book { void cast() { shine(); } void shine() { return; } };","C++")];
    for (path, content, language) in cases {
        let a = analyze(&[file(path, content)]);
        assert_eq!(
            a.entities.iter().filter(|e| e.kind == "function").count(),
            2,
            "{path}: {:?}",
            a.entities
        );
        assert!(a.entities.iter().all(|e| e.language == language));
        assert_eq!(
            a.edges.iter().filter(|e| e.kind == "calls").count(),
            1,
            "{path}"
        );
    }
}
#[test]
fn ambiguous_calls_are_not_falsely_connected() {
    let a = analyze(&[
        file("main.rs", "fn caller() { duplicate(); }"),
        file("a.rs", "fn duplicate() {}"),
        file("b.rs", "fn duplicate() {}"),
    ]);
    assert!(a.edges.is_empty());
    assert_eq!(a.unresolved_calls, 1);
}
#[test]
fn threshold_does_not_remove_functions_and_svg_escapes_input() {
    let a = analyze(&[file("a<&.rs", "fn short() { return; }")]);
    let d = render(&a, "<script>alert('x')</script>", 12, "gold");
    assert_eq!(d.placements.len(), 1);
    assert_eq!(d.placements[0].radius, 66.);
    assert!(!d.svg.contains("<script>"));
    assert!(d.svg.contains("&lt;script&gt;"));
    assert!(d.svg.contains("a&lt;&amp;.rs"));
    assert_eq!(
        d.svg,
        render(&a, "<script>alert('x')</script>", 12, "gold").svg
    );
}
#[test]
fn no_declarations_and_unsupported_extensions_are_explicit() {
    let a = analyze(&[file("a.txt", "fn hidden() {}"), file("a.py", "x = 1")]);
    assert_eq!(a.files, 1);
    assert_eq!(a.warnings.len(), 1);
    assert!(render(&a, "empty", 12, "gold")
        .svg
        .contains("No supported declarations found"));
}
#[test]
fn calls_in_conditions_are_not_declarations() {
    let a=analyze(&[file("main.rs","fn caller() { if check() { work(); } while check() { work(); } } fn check() { return true; } fn work() { return; }")]);
    assert_eq!(
        a.entities.iter().filter(|e| e.kind == "function").count(),
        3
    );
}

#[test]
fn class_fields_do_not_include_method_locals() {
    let a = analyze(&[file(
        "Book.java",
        "class Book { int pages; void open() { int brightness = 2; } }",
    )]);
    let book = a.entities.iter().find(|e| e.kind == "class").unwrap();
    assert!(book.fields.contains(&"pages".into()));
    assert!(!book.fields.contains(&"brightness".into()));
}
#[test]
fn unrelated_languages_do_not_link_by_name() {
    let a = analyze(&[
        file("a.rs", "fn cast() { sparkle(); }"),
        file("b.py", "def sparkle():\n    return 1\n"),
    ]);
    assert!(a.edges.is_empty());
    assert_eq!(a.unresolved_calls, 1);
}

#[test]
fn javascript_strings_do_not_behave_like_rust_lifetimes() {
    let a = analyze(&[file("app.js", "const title = 'magic'; function cast() { const name = 'runes'; spark(); } function spark() { return 'glow'; }")]);
    assert_eq!(
        a.entities.iter().filter(|e| e.kind == "function").count(),
        2
    );
    assert_eq!(a.edges.iter().filter(|e| e.kind == "calls").count(), 1);
}
#[test]
fn rust_raw_strings_hide_embedded_source() {
    let content = "fn real() { let raw = r##\"fn illusion() { let name = \"fake\"; bad(); }\"##; let bytes = br#\"fn ghost() {}\"#; helper(); } fn helper() { return; }";
    let a = analyze(&[file("raw.rs", content)]);
    assert_eq!(
        a.entities.iter().filter(|e| e.kind == "function").count(),
        2
    );
    assert_eq!(a.edges.iter().filter(|e| e.kind == "calls").count(), 1);
    assert!(!a
        .entities
        .iter()
        .any(|e| e.name == "illusion" || e.name == "ghost"));
}
