use base64::Engine;
use clap::Parser;
use code_is_magic::{
    analyze, compact_svg, decode_image_with_key, decode_svg_names_with_key, embed_svg_with_key,
    encode_magic_with_key, encode_svgz_with_key, optimize_png, render_with_key, snapshot_zip,
    ArchiveEntry, Capsule, MagicKey, Snapshot,
};
use reqwest::blocking::Client;
use serde_json::Value;
use std::{
    error::Error,
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
    time::Duration,
};
use zeroize::Zeroizing;

#[derive(Parser)]
#[command(
    version,
    about = "Transmute a repository into magic circles, or recover source from a magic image"
)]
struct Args {
    /// GitHub repository URL, local source directory, or recoverable legacy/canonical image
    #[arg(value_name = "INPUT")]
    repository: Option<String>,
    /// Losslessly optimize an existing legacy PNG, SVG, or SVGZ; this is migration, not transmutation
    #[arg(long, conflicts_with_all = ["repository", "decode", "decode_names", "generate_key"])]
    optimize: Option<PathBuf>,
    /// Recover the complete snapshot from a canonical SVGZ or legacy PNG/SVG/SVGZ into a ZIP
    #[arg(long)]
    decode: Option<PathBuf>,
    /// Recover names from geometric contours in canonical SVGZ or legacy SVG/PNG
    #[arg(long)]
    decode_names: Option<PathBuf>,
    /// Canonical transmutation output. Code-to-magic emits SVGZ only.
    #[arg(short, long, default_value = "magic.svgz")]
    output: PathBuf,
    /// Minimum non-comment source lines for full ornament detail; size always follows code lines
    #[arg(long, default_value_t = 12)]
    min_lines: usize,
    #[arg(long,default_value="gold",value_parser=["gold","astral","parchment"])]
    palette: String,
    /// Use portable vector dingbat ornament (legacy --wingdings flag)
    #[arg(long)]
    wingdings: bool,
    /// Color seals using their language's color; tones adapt to the selected background
    #[arg(long)]
    language_colors: bool,
    /// Read a 64-hex-digit magic key from a file, or '-' for stdin; overrides CODE_IS_MAGIC_KEY
    #[arg(long)]
    key_file: Option<PathBuf>,
    /// Generate a random magic key into a new file (keep it outside the captured source folder)
    #[arg(long, conflicts_with = "key_file")]
    generate_key: Option<PathBuf>,
    /// Branch, tag, or commit; defaults to the repository's default branch
    #[arg(long)]
    reference: Option<String>,
    /// Maximum tracked snapshot entries (up to 5,000); exceeding the limit fails
    #[arg(long, default_value_t = 5000)]
    max_files: usize,
}
fn repo_slug(input: &str) -> Result<String, Box<dyn Error>> {
    let s = input
        .strip_prefix("https://github.com/")
        .ok_or("Expected an HTTPS github.com repository URL or local directory")?
        .trim_end_matches('/')
        .trim_end_matches(".git");
    let parts: Vec<_> = s.split('/').collect();
    if parts.len() != 2
        || parts.iter().any(|p| {
            p.is_empty()
                || !p
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
                || matches!(*p, "." | "..")
        })
    {
        return Err("Use the repository root URL, without query strings or a /tree/ suffix; choose revisions with --reference".into());
    }
    Ok(s.into())
}
fn api(client: &Client, path: &str, token: &Option<String>) -> Result<Value, Box<dyn Error>> {
    let mut request = client
        .get(format!("https://api.github.com/{path}"))
        .header("Accept", "application/vnd.github+json");
    if let Some(t) = token {
        request = request.bearer_auth(t);
    }
    let response = request.send()?;
    let status = response.status();
    if !status.is_success() {
        return Err(format!("GitHub returned {status}. Check repository access and GITHUB_TOKEN; unauthenticated requests have a low rate limit.").into());
    }
    Ok(response.json()?)
}
fn trees(
    client: &Client,
    slug: &str,
    sha: &str,
    prefix: &str,
    token: &Option<String>,
    out: &mut Vec<Value>,
) -> Result<(), Box<dyn Error>> {
    let tree = api(client, &format!("repos/{slug}/git/trees/{sha}"), token)?;
    if tree["truncated"].as_bool().unwrap_or(false) {
        return Err("GitHub truncated a directory; no partial snapshot will be generated".into());
    }
    for e in tree["tree"].as_array().ok_or("No tree")? {
        let path = format!("{prefix}{}", e["path"].as_str().ok_or("Invalid path")?);
        if e["type"] == "tree" {
            trees(
                client,
                slug,
                e["sha"].as_str().ok_or("Missing SHA")?,
                &format!("{path}/"),
                token,
                out,
            )?;
        } else {
            let mut e = e.clone();
            e["path"] = Value::String(path);
            out.push(e);
        }
        if out.len() > 5000 {
            return Err("Snapshot exceeds 5,000 entries".into());
        }
    }
    Ok(())
}
fn github(input: &str, args: &Args) -> Result<Snapshot, Box<dyn Error>> {
    let slug = repo_slug(input)?;
    let token = std::env::var("GITHUB_TOKEN").ok();
    let client = Client::builder()
        .user_agent("Code-Is-Magic/0.2")
        .timeout(Duration::from_secs(40))
        .build()?;
    let meta = api(&client, &format!("repos/{slug}"), &token)?;
    let reference = args
        .reference
        .clone()
        .unwrap_or_else(|| meta["default_branch"].as_str().unwrap_or("main").into());
    let encoded: String = reference
        .bytes()
        .map(|b| {
            if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.') {
                (b as char).to_string()
            } else {
                format!("%{b:02X}")
            }
        })
        .collect();
    let commit = api(&client, &format!("repos/{slug}/commits/{encoded}"), &token)?;
    let sha = commit["sha"].as_str().ok_or("Missing commit SHA")?;
    let root = commit["commit"]["tree"]["sha"]
        .as_str()
        .ok_or("Missing tree")?;
    let tree = api(
        &client,
        &format!("repos/{slug}/git/trees/{root}?recursive=1"),
        &token,
    )?;
    let mut entries = Vec::new();
    if tree["truncated"].as_bool().unwrap_or(false) {
        trees(&client, &slug, root, "", &token, &mut entries)?;
    } else {
        entries = tree["tree"]
            .as_array()
            .ok_or("No source tree")?
            .iter()
            .filter(|e| e["type"] != "tree")
            .cloned()
            .collect();
    }
    if entries.len() > args.max_files.min(5000) {
        return Err("Repository entry limit exceeded".into());
    }
    entries.sort_by(|a, b| a["path"].as_str().cmp(&b["path"].as_str()));
    let mut snapshot = Snapshot {
        version: 1,
        title: format!("{slug} @ {}", &sha[..12]),
        commit: Some(sha.into()),
        entries: Vec::new(),
    };
    let mut bytes = 0;
    for (i, e) in entries.iter().enumerate() {
        let path = e["path"].as_str().ok_or("Missing path")?;
        let blob = e["sha"].as_str().ok_or("Missing SHA")?;
        let mode = e["mode"].as_str().ok_or("Missing file mode")?;
        if e["type"] == "commit" {
            snapshot.entries.push(ArchiveEntry {
                path: path.into(),
                mode: "160000".into(),
                data: String::new(),
                sha: Some(blob.into()),
            });
            continue;
        }
        let size = e["size"].as_u64().unwrap_or(0) as usize;
        if size > 2_000_000 || bytes + size > 50_000_000 {
            return Err("Repository size budget exceeded; no files omitted".into());
        }
        eprintln!("Reading {}/{}: {path}", i + 1, entries.len());
        let data = api(&client, &format!("repos/{slug}/git/blobs/{blob}"), &token)?;
        if data["encoding"] != "base64" {
            return Err("Unsupported blob encoding".into());
        }
        let data = data["content"]
            .as_str()
            .ok_or("Missing file bytes")?
            .split_whitespace()
            .collect::<String>();
        bytes += base64::engine::general_purpose::STANDARD
            .decode(&data)?
            .len();
        snapshot.entries.push(ArchiveEntry {
            path: path.into(),
            mode: mode.into(),
            data,
            sha: Some(blob.into()),
        });
    }
    snapshot.canonicalize()?;
    Ok(snapshot)
}
fn local(
    path: &Path,
    root: &Path,
    snapshot: &mut Snapshot,
    limit: usize,
) -> Result<(), Box<dyn Error>> {
    let mut entries = fs::read_dir(path)?.collect::<Result<Vec<_>, _>>()?;
    entries.sort_by_key(|e| e.path());
    for e in entries {
        let p = e.path();
        let ty = e.file_type()?;
        if ty.is_dir() {
            if !matches!(
                e.file_name().to_str(),
                Some(".git" | "target" | "node_modules" | "pkg")
            ) {
                local(&p, root, snapshot, limit)?;
            }
            continue;
        }
        if snapshot.entries.len() >= limit.min(5000) {
            return Err("Local snapshot entry limit exceeded".into());
        }
        let (bytes, mode) = if ty.is_symlink() {
            (
                fs::read_link(&p)?.as_os_str().as_encoded_bytes().to_vec(),
                "120000",
            )
        } else {
            if e.metadata()?.len() > 2_000_000 {
                return Err("Local file exceeds 2 MB".into());
            }
            #[cfg(unix)]
            let mode = {
                use std::os::unix::fs::PermissionsExt;
                if e.metadata()?.permissions().mode() & 0o111 != 0 {
                    "100755"
                } else {
                    "100644"
                }
            };
            #[cfg(not(unix))]
            let mode = "100644";
            (fs::read(&p)?, mode)
        };
        snapshot.entries.push(ArchiveEntry {
            path: p
                .strip_prefix(root)?
                .to_str()
                .ok_or("Path is not UTF-8")?
                .replace('\\', "/"),
            mode: mode.into(),
            data: base64::engine::general_purpose::STANDARD.encode(bytes),
            sha: None,
        });
    }
    Ok(())
}
fn read_capsule(path: &Path, key: Option<&MagicKey>) -> Result<Capsule, Box<dyn Error>> {
    Ok(decode_image_with_key(&fs::read(path)?, key)?)
}
fn run() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();
    if let Some(path) = &args.optimize {
        if path == &args.output
            || fs::canonicalize(&args.output)
                .ok()
                .is_some_and(|output| fs::canonicalize(path).ok() == Some(output))
        {
            return Err("Choose a separate output path to preserve the original image".into());
        }
        let bytes = fs::read(path)?;
        if bytes.starts_with(b"\x89PNG") {
            if !args.output.extension().is_some_and(|e| e == "png") {
                return Err("Use a .png output path for pixel-preserving legacy PNG optimization".into());
            }
            let optimized = optimize_png(&bytes)?;
            fs::write(&args.output, &optimized)?;
            println!(
                "Lossless legacy PNG: {} → {} bytes; every pixel and source bit preserved",
                bytes.len(),
                optimized.len()
            );
            return Ok(());
        }
    }
    if let Some(path) = &args.generate_key {
        let mut bytes = Zeroizing::new([0_u8; 32]);
        getrandom::getrandom(bytes.as_mut()).map_err(|_| "Secure key generation failed")?;
        let text = Zeroizing::new(bytes.iter().map(|b| format!("{b:02x}")).collect::<String>());
        let mut options = fs::OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut file = options.open(path)?;
        writeln!(file, "{}", text.as_str())?;
        println!(
            "Generated magic key in {}. Keep this file to recover keyed images.",
            path.display()
        );
        return Ok(());
    }
    let key_text = if let Some(path) = &args.key_file {
        let mut text = Zeroizing::new(String::new());
        if path == Path::new("-") {
            std::io::stdin().take(1025).read_to_string(&mut text)?;
        } else {
            fs::File::open(path)?.take(1025).read_to_string(&mut text)?;
        }
        Some(text)
    } else {
        match std::env::var("CODE_IS_MAGIC_KEY") {
            Ok(text) => Some(Zeroizing::new(text)),
            Err(std::env::VarError::NotPresent) => None,
            Err(_) => return Err("CODE_IS_MAGIC_KEY must contain a UTF-8 hexadecimal key".into()),
        }
    };
    let key = key_text
        .as_deref()
        .map(|text| MagicKey::from_hex(text))
        .transpose()?;
    drop(key_text);
    if let Some(path) = &args.optimize {
        if !args
            .output
            .extension()
            .is_some_and(|e| e == "svg" || e == "svgz")
        {
            return Err("Choose a .svg or .svgz output path for legacy vector optimization".into());
        }
        let mut capsule = read_capsule(path, key.as_ref())?;
        capsule.svg = compact_svg(&capsule.svg)?;
        let bytes = if args.output.extension().is_some_and(|e| e == "svgz") {
            encode_svgz_with_key(&capsule, key.as_ref())?
        } else {
            embed_svg_with_key(&capsule, key.as_ref())?.into_bytes()
        };
        fs::write(&args.output, &bytes)?;
        println!(
            "Lossless legacy vector: {} → {} bytes; exact outlines and source preserved",
            fs::metadata(path)?.len(),
            bytes.len()
        );
        return Ok(());
    }
    let image_input = args.decode.clone().or_else(|| {
        if args.decode_names.is_some() {
            None
        } else {
            args.repository
                .as_deref()
                .filter(|input| Path::new(input).is_file())
                .map(PathBuf::from)
        }
    });
    if let Some(path) = &image_input {
        let capsule = read_capsule(path, key.as_ref())?;
        let output = if args.output == Path::new("magic.svgz") {
            PathBuf::from("recovered.zip")
        } else {
            args.output.clone()
        };
        fs::write(&output, snapshot_zip(&capsule.snapshot)?)?;
        println!(
            "Recovered {} tracked entries with exact bytes and modes to {}",
            capsule.snapshot.entries.len(),
            output.display()
        );
        return Ok(());
    }
    if let Some(path) = &args.decode_names {
        let bytes = fs::read(path)?;
        let svg = if bytes.starts_with(b"\x89PNG") || bytes.starts_with(b"\x1f\x8b") {
            decode_image_with_key(&bytes, key.as_ref())?.svg
        } else {
            String::from_utf8(bytes)?
        };
        println!(
            "{}",
            serde_json::to_string_pretty(&decode_svg_names_with_key(&svg, key.as_ref())?)?
        );
        return Ok(());
    }
    let input=args.repository.as_deref().ok_or("Provide a GitHub repository, local directory, canonical .svgz, legacy recoverable image, or --decode-names image")?;
    if Path::new(input).is_dir() {
        if let Some(path) = args
            .key_file
            .as_ref()
            .filter(|p| p.as_path() != Path::new("-"))
        {
            if path
                .canonicalize()?
                .starts_with(Path::new(input).canonicalize()?)
            {
                return Err("Keep the magic key file outside the captured source directory".into());
            }
        }
    }
    let mut snapshot = if Path::new(input).is_dir() {
        let mut s = Snapshot {
            version: 1,
            title: input.into(),
            commit: None,
            entries: Vec::new(),
        };
        local(Path::new(input), Path::new(input), &mut s, args.max_files)?;
        s
    } else {
        github(input, &args)?
    };
    snapshot.canonicalize()?;
    let analysis = analyze(&snapshot.source_files());
    let diagram = render_with_key(
        &analysis,
        &snapshot.title,
        args.min_lines.max(1),
        &args.palette,
        args.wingdings,
        args.language_colors,
        key.as_ref(),
    )?;
    let capsule = Capsule {
        version: 1,
        snapshot: snapshot.clone(),
        svg: diagram.svg.clone(),
    };
    if !args.output.extension().is_some_and(|e| e == "svgz") {
        return Err("Code-to-magic transmutation has one canonical format: choose a .svgz output path".into());
    }
    fs::write(&args.output, encode_magic_with_key(&capsule, key.as_ref())?)?;
    println!(
        "✦ CODE IS MAGIC ✦\n{} source files · {} declarations · {} call channels",
        analysis.files,
        diagram.placements.len(),
        analysis.edges.iter().filter(|e| e.kind == "calls").count()
    );
    println!(
        "{} complete tracked entries embedded byte-exactly",
        snapshot.entries.len()
    );
    println!(
        "{} unresolved/external calls · structural analysis",
        analysis.unresolved_calls
    );
    println!("Canonical perceptual SVGZ written to {}", args.output.display());
    Ok(())
}
fn main() {
    if let Err(e) = run() {
        eprintln!("Transmutation failed: {e}");
        let mut cause = e.source();
        while let Some(error) = cause {
            eprintln!("  caused by: {error}");
            cause = error.source();
        }
        std::process::exit(1);
    }
}
