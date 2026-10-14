// Rust-specific lexical, declaration and forge design rules.
fn quoted(value: &str) -> String {
    format!("\"{}\"", value.replace('\\', "\\\\").replace('"', "\\\""))
}
fn main() {
    let name = "Rust";
    let keyword = "fn";
    let path = "M -.024 -.94 L 0 -.99 L .024 -.94 M 0 -.99 L 0 -.90";
    println!(
        concat!(
            "{{\"id\":\"rust\",\"base\":\"c\",\"name\":{},\"extensions\":[\"rs\"],",
            "\"call_group\":\"rust\",\"raw_strings\":true,\"lifetimes\":true,",
            "\"required_function_keyword\":{},\"color\":\"#dea584\",\"sigil\":\"ᛟ\",\"repeats\":8,",
            "\"collar\":[{{\"op\":\"path\",\"data\":{}}}]}}"
        ),
        quoted(name),
        quoted(keyword),
        quoted(path)
    );
}
