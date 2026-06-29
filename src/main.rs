use ignore::WalkBuilder;
use std::collections::BTreeSet;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

const OUTPUT_FILE: &str = "context.md";

const EXCLUDED_FILENAMES: [&str; 13] = [
    "Cargo.lock",
    "package-lock.json",
    "yarn.lock",
    "pnpm-lock.yaml",
    "bun.lockb",
    "composer.lock",
    "Gemfile.lock",
    "poetry.lock",
    "Pipfile.lock",
    "flake.lock",
    "mix.lock",
    "packages.lock.json",
    "go.sum",
];

const TEXT_EXTENSIONS: [&str; 84] = [
    "c", "h", "cpp", "cc", "cxx", "hpp", "hxx",
    "rs", "go", "py", "js", "ts", "jsx", "tsx",
    "java", "kt", "swift", "rb", "php", "cs", "fs", "fsx",
    "html", "htm", "css", "scss", "sass", "less",
    "sh", "bash", "zsh", "fish", "ps1",
    "toml", "yaml", "yml", "json", "jsonc", "xml",
    "md", "mdx", "txt", "rst", "csv", "sql",
    "env", "gitignore", "gitattributes", "editorconfig",
    "lock", "conf", "cfg", "ini", "properties",
    "makefile", "dockerfile", "containerfile",
    "lua", "vim", "el", "lisp", "clj", "cljs",
    "ex", "exs", "erl", "hrl",
    "hs", "lhs", "ml", "mli",
    "r", "jl", "scala",
    "proto", "thrift", "graphql", "gql",
    "tf", "tfvars", "hcl",
    "svelte", "vue", "astro",
];

const BINARY_EXTENSIONS: [&str; 53] = [
    "png", "jpg", "jpeg", "gif", "bmp", "ico", "svg", "webp", "tiff", "tif",
    "mp3", "mp4", "wav", "ogg", "flac", "avi", "mov", "mkv", "webm",
    "pdf", "doc", "docx", "xls", "xlsx", "ppt", "pptx",
    "zip", "tar", "gz", "bz2", "xz", "7z", "rar",
    "exe", "dll", "so", "dylib", "a", "o", "wasm",
    "ttf", "otf", "woff", "woff2", "eot",
    "db", "sqlite", "sqlite3",
    "bin", "dat", "class", "pyc", "pyd",
];

fn extension(path: &Path) -> Option<&str> {
    path.extension()?.to_str()
}

fn is_excluded_filename(path: &Path) -> bool {
    let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
        return false;
    };
    if EXCLUDED_FILENAMES.iter().any(|&e| e == name) {
        return true;
    }
    // Minified files: foo.min.js, bar.min.css, etc.
    path.file_stem()
        .and_then(|s| s.to_str())
        .map(|s| s.ends_with(".min"))
        .unwrap_or(false)
}

fn is_binary_extension(path: &Path) -> bool {
    extension(path).map_or(false, |ext| {
        let lower = ext.to_lowercase();
        BINARY_EXTENSIONS.iter().any(|&e| e == lower)
    })
}

fn is_text_extension(path: &Path) -> bool {
    extension(path).map_or(false, |ext| {
        let lower = ext.to_lowercase();
        TEXT_EXTENSIONS.iter().any(|&e| e == lower)
    })
}

/// Sniff the first 8 KB for NUL bytes — fast binary detection.
fn is_binary_content(path: &Path) -> bool {
    let Ok(bytes) = fs::read(path) else { return true };
    let sample = &bytes[..bytes.len().min(8192)];
    sample.contains(&0u8)
}

fn lang_hint(path: &Path) -> &str {
    match extension(path).unwrap_or("").to_lowercase().as_str() {
        "rs" => "rust",
        "py" => "python",
        "js" | "jsx" => "javascript",
        "ts" | "tsx" => "typescript",
        "go" => "go",
        "c" | "h" => "c",
        "cpp" | "cc" | "cxx" | "hpp" | "hxx" => "cpp",
        "java" => "java",
        "kt" => "kotlin",
        "swift" => "swift",
        "rb" => "ruby",
        "php" => "php",
        "cs" => "csharp",
        "sh" | "bash" | "zsh" | "fish" => "bash",
        "ps1" => "powershell",
        "html" | "htm" => "html",
        "css" => "css",
        "scss" => "scss",
        "toml" => "toml",
        "yaml" | "yml" => "yaml",
        "json" | "jsonc" => "json",
        "xml" => "xml",
        "md" | "mdx" => "markdown",
        "sql" => "sql",
        "lua" => "lua",
        "ex" | "exs" => "elixir",
        "erl" | "hrl" => "erlang",
        "hs" | "lhs" => "haskell",
        "ml" | "mli" => "ocaml",
        "r" => "r",
        "jl" => "julia",
        "scala" => "scala",
        "proto" => "protobuf",
        "graphql" | "gql" => "graphql",
        "tf" | "tfvars" | "hcl" => "hcl",
        "svelte" => "svelte",
        "vue" => "vue",
        "dockerfile" | "containerfile" => "dockerfile",
        _ => "",
    }
}

fn collect_files(root: &Path) -> Vec<PathBuf> {
    let output_canonical = root.join(OUTPUT_FILE).canonicalize().ok();

    let mut files: BTreeSet<PathBuf> = BTreeSet::new();

    let walker = WalkBuilder::new(root)
        .hidden(false)
        .git_ignore(true)
        .git_global(true)
        .git_exclude(true)
        .require_git(false)
        .add_custom_ignore_filename(".ctxignore")
        .build();

    for result in walker {
        let entry = match result {
            Ok(e) => e,
            Err(e) => {
                eprintln!("Warning: {e}");
                continue;
            }
        };

        if entry.file_type().map(|ft| !ft.is_file()).unwrap_or(true) {
            continue;
        }

        let path = entry.path().to_path_buf();

        // Skip .git internals.
        if path.components().any(|c| c.as_os_str() == ".git") {
            continue;
        }

        // Skip the output file itself.
        if let Some(ref canon) = output_canonical {
            if path.canonicalize().ok().as_ref() == Some(canon) {
                continue;
            }
        }

        // Skip lock files and other excluded filenames.
        if is_excluded_filename(&path) {
            continue;
        }

        // Skip by extension first (cheap).
        if is_binary_extension(&path) {
            continue;
        }

        // Skip if not a known text extension AND binary content.
        if !is_text_extension(&path) && is_binary_content(&path) {
            continue;
        }

        files.insert(path);
    }

    files.into_iter().collect()
}

fn write_tree(out: &mut impl Write, files: &[PathBuf], root: &Path) -> io::Result<()> {
    writeln!(out, "## Árbol de archivos\n")?;
    writeln!(out, "```")?;
    for path in files {
        let rel = path.strip_prefix(root).unwrap_or(path);
        writeln!(out, "{}", rel.display())?;
    }
    writeln!(out, "```\n")?;
    Ok(())
}

fn write_file_section(out: &mut impl Write, path: &Path, root: &Path) -> io::Result<()> {
    let rel = path.strip_prefix(root).unwrap_or(path);
    let content = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Warning: cannot read {}: {e}", path.display());
            return Ok(());
        }
    };

    let lang = lang_hint(path);
    writeln!(out, "## Archivo: {}\n", rel.display())?;
    writeln!(out, "```{lang}")?;
    // Escape any closing fence that would break the markdown block.
    let safe = content.replace("```", "` ` `");
    write!(out, "{safe}")?;
    if !safe.ends_with('\n') {
        writeln!(out)?;
    }
    writeln!(out, "```\n")?;
    Ok(())
}

fn main() -> io::Result<()> {
    let root = std::env::current_dir()?;
    let output_path = root.join(OUTPUT_FILE);

    let files = collect_files(&root);

    if files.is_empty() {
        eprintln!("No se encontraron archivos de texto en {}", root.display());
        return Ok(());
    }

    println!(
        "Generando {} con {} archivo(s)…",
        OUTPUT_FILE,
        files.len()
    );

    let mut buf: Vec<u8> = Vec::new();

    writeln!(
        buf,
        "# Contexto del proyecto: {}\n",
        root.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("proyecto")
    )?;
    writeln!(
        buf,
        "> Generado automáticamente por **ctx-gen**. No editar manualmente.\n"
    )?;

    write_tree(&mut buf, &files, &root)?;

    writeln!(buf, "---\n")?;
    writeln!(buf, "## Contenido de archivos\n")?;

    for path in &files {
        write_file_section(&mut buf, path, &root)?;
    }

    fs::write(&output_path, &buf)?;
    println!("✓ {} generado ({} bytes)", OUTPUT_FILE, buf.len());

    Ok(())
}
