use base64::Engine;
use clap::Parser;
use code_is_magic::{analyze, render_with_wingdings, supported, SourceFile};
use reqwest::blocking::Client;
use serde_json::Value;
use std::{
    error::Error,
    fs,
    path::{Path, PathBuf},
    time::Duration,
};

#[derive(Parser)]
#[command(
    version,
    about = "Transmute a GitHub repository or local source directory into magic circles"
)]
struct Args {
    /// https://github.com/owner/repo[.git], or a local directory
    repository: String,
    #[arg(short, long, default_value = "magic.svg")]
    output: PathBuf,
    /// Minimum non-comment source lines for a full circle; smaller functions become compact seals
    #[arg(long, default_value_t = 12)]
    min_lines: usize,
    #[arg(long,default_value="gold",value_parser=["gold","astral","parchment"])]
    palette: String,
    /// Use the locally installed Wingdings font for supplementary ornament (not bundled)
    #[arg(long)]
    wingdings: bool,
    /// Branch, tag, or commit; defaults to the repository's default branch
    #[arg(long)]
    reference: Option<String>,
    /// Maximum supported source files; exceeding the limit fails rather than truncates
    #[arg(long, default_value_t = 5000)]
    max_files: usize,
    /// Save a JSON diagram for offline browser import
    #[arg(long)]
    json: Option<PathBuf>,
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
    out: &mut Vec<(String, String, usize)>,
    limit: usize,
) -> Result<(), Box<dyn Error>> {
    let tree = api(client, &format!("repos/{slug}/git/trees/{sha}"), token)?;
    let entries = tree["tree"]
        .as_array()
        .ok_or("GitHub returned no source tree")?;
    for entry in entries {
        let path = format!(
            "{prefix}{}",
            entry["path"].as_str().ok_or("Invalid tree path")?
        );
        let sha = entry["sha"].as_str().ok_or("Invalid tree SHA")?;
        match entry["type"].as_str() {
            Some("tree") => trees(client, slug, sha, &format!("{path}/"), token, out, limit)?,
            Some("blob") if supported(&path) => {
                out.push((
                    path,
                    sha.into(),
                    entry["size"].as_u64().unwrap_or(0) as usize,
                ));
                if out.len() > limit {
                    return Err("Source file limit exceeded. Increase --max-files or select a smaller repository.".into());
                }
            }
            Some("commit") => eprintln!("Submodule excluded: {path} (an independent repository)"),
            _ => {}
        }
    }
    Ok(())
}
fn github(input: &str, args: &Args) -> Result<(Vec<SourceFile>, String), Box<dyn Error>> {
    let slug = repo_slug(input)?;
    let token = std::env::var("GITHUB_TOKEN").ok();
    let client = Client::builder()
        .user_agent("Code-Is-Magic/0.1")
        .timeout(Duration::from_secs(40))
        .build()?;
    let metadata = api(&client, &format!("repos/{slug}"), &token)?;
    let reference = args
        .reference
        .clone()
        .unwrap_or_else(|| metadata["default_branch"].as_str().unwrap_or("main").into());
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
    let sha = commit["sha"].as_str().ok_or("Revision has no commit")?;
    let root = commit["commit"]["tree"]["sha"]
        .as_str()
        .ok_or("Commit has no tree")?;
    let recursive = api(
        &client,
        &format!("repos/{slug}/git/trees/{root}?recursive=1"),
        &token,
    )?;
    let mut entries = Vec::new();
    if recursive["truncated"].as_bool().unwrap_or(false) {
        trees(
            &client,
            &slug,
            root,
            "",
            &token,
            &mut entries,
            args.max_files,
        )?;
    } else {
        for e in recursive["tree"].as_array().ok_or("No source tree")? {
            let path = e["path"].as_str().unwrap_or("");
            if e["type"] == "blob" && supported(path) {
                entries.push((
                    path.into(),
                    e["sha"].as_str().ok_or("Missing blob SHA")?.into(),
                    e["size"].as_u64().unwrap_or(0) as usize,
                ));
            }
        }
    }
    if entries.len() > args.max_files {
        return Err("Source file limit exceeded; increase --max-files".into());
    }
    entries.sort();
    let mut files = Vec::new();
    let mut bytes = 0;
    for (i, (path, blob, size)) in entries.iter().enumerate() {
        if *size > 2_000_000 || bytes + size > 50_000_000 {
            return Err(
                format!("Source budget exceeded at {path} (2 MB per file / 50 MB total)").into(),
            );
        }
        eprintln!("Reading {}/{}: {}", i + 1, entries.len(), path);
        let data = api(&client, &format!("repos/{slug}/git/blobs/{blob}"), &token)?;
        if data["encoding"] != "base64" {
            return Err(format!("Unsupported blob encoding: {path}").into());
        }
        let content = base64::engine::general_purpose::STANDARD.decode(
            data["content"]
                .as_str()
                .ok_or("Missing source content")?
                .split_whitespace()
                .collect::<String>(),
        )?;
        bytes += content.len();
        files.push(SourceFile {
            path: path.clone(),
            content: String::from_utf8(content)?,
        });
    }
    Ok((files, format!("{slug} @ {}", &sha[..12])))
}
fn local(
    path: &Path,
    root: &Path,
    files: &mut Vec<SourceFile>,
    limit: usize,
    bytes: &mut usize,
) -> Result<(), Box<dyn Error>> {
    let mut entries = fs::read_dir(path)?.collect::<Result<Vec<_>, _>>()?;
    entries.sort_by_key(|e| e.path());
    for entry in entries {
        let p = entry.path();
        let ty = entry.file_type()?;
        if ty.is_symlink() {
            continue;
        }
        if ty.is_dir() {
            if entry.file_name() != ".git"
                && entry.file_name() != "target"
                && entry.file_name() != "node_modules"
            {
                local(&p, root, files, limit, bytes)?;
            }
        } else if supported(&p.to_string_lossy()) {
            if files.len() >= limit {
                return Err("Source file limit exceeded".into());
            }
            let size = entry.metadata()?.len() as usize;
            if size > 2_000_000 || *bytes + size > 50_000_000 {
                return Err("Source size budget exceeded".into());
            }
            *bytes += size;
            files.push(SourceFile {
                path: p.strip_prefix(root)?.to_string_lossy().replace('\\', "/"),
                content: fs::read_to_string(p)?,
            });
        }
    }
    Ok(())
}
fn run() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();
    let (files, title) = if Path::new(&args.repository).is_dir() {
        let mut files = Vec::new();
        local(
            Path::new(&args.repository),
            Path::new(&args.repository),
            &mut files,
            args.max_files,
            &mut 0,
        )?;
        (files, args.repository.clone())
    } else {
        github(&args.repository, &args)?
    };
    let analysis = analyze(&files);
    let diagram = render_with_wingdings(
        &analysis,
        &title,
        args.min_lines.max(1),
        &args.palette,
        args.wingdings,
    );
    fs::write(&args.output, &diagram.svg)?;
    if let Some(path) = args.json {
        fs::write(path, serde_json::to_string_pretty(&diagram)?)?;
    }
    println!(
        "✦ CODE IS MAGIC ✦\n{} files · {} declarations · {} call channels",
        analysis.files,
        analysis
            .entities
            .iter()
            .filter(|e| e.kind != "merged_impl")
            .count(),
        analysis.edges.iter().filter(|e| e.kind == "calls").count()
    );
    println!(
        "{} unresolved/external calls · structural analysis",
        analysis.unresolved_calls
    );
    println!("Circle written to {}", args.output.display());
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
