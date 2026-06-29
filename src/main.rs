use ignore::WalkBuilder;
use std::collections::BTreeSet;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use zip::write::FileOptions;
use zip::ZipWriter;

const OUTPUT_MD: &str = "context.md";
const OUTPUT_ZIP: &str = "context.zip";
const MAX_LINES_PER_FILE: usize = 1000;

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
    path.file_stem()
        .and_then(|s| s.to_str())
        .map(|s| s.ends_with(".min"))
        .unwrap_or(false)
}

fn is_ctx_output(path: &Path, output_md_canon: &Option<PathBuf>) -> bool {
    if let Some(ref canon) = output_md_canon {
        if path.canonicalize().ok().as_ref() == Some(canon) {
            return true;
        }
    }
    let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
        return false;
    };
    // Skip context-1.md, context-2.md, etc.
    if let Some(rest) = name.strip_prefix("context-") {
        if let Some(num) = rest.strip_suffix(".md") {
            if !num.is_empty() && num.chars().all(|c| c.is_ascii_digit()) {
                return true;
            }
        }
    }
    false
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
    let output_md_canon = root.join(OUTPUT_MD).canonicalize().ok();
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
            Err(e) => { eprintln!("Warning: {e}"); continue; }
        };

        if entry.file_type().map(|ft| !ft.is_file()).unwrap_or(true) {
            continue;
        }

        let path = entry.path().to_path_buf();

        if path.components().any(|c| c.as_os_str() == ".git") {
            continue;
        }

        if is_ctx_output(&path, &output_md_canon) {
            continue;
        }

        if is_excluded_filename(&path) {
            continue;
        }

        if is_binary_extension(&path) {
            continue;
        }

        if !is_text_extension(&path) && is_binary_content(&path) {
            continue;
        }

        files.insert(path);
    }

    files.into_iter().collect()
}

fn build_header(root: &Path, files: &[PathBuf]) -> String {
    let project = root
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("proyecto");

    let mut out = String::new();
    out.push_str(&format!("# Contexto del proyecto: {project}\n\n"));
    out.push_str("> Generado automáticamente por **ctx-gen**. No editar manualmente.\n\n");

    out.push_str("## Árbol de archivos\n\n```\n");
    for path in files {
        let rel = path.strip_prefix(root).unwrap_or(path);
        out.push_str(&format!("{}\n", rel.display()));
    }
    out.push_str("```\n\n---\n\n## Contenido de archivos\n\n");
    out
}

fn build_file_section(path: &Path, root: &Path) -> String {
    let rel = path.strip_prefix(root).unwrap_or(path);
    let content = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Warning: cannot read {}: {e}", path.display());
            return String::new();
        }
    };

    let lang = lang_hint(path);
    let safe = content.replace("```", "` ` `");
    let trailing = if safe.ends_with('\n') { "" } else { "\n" };
    format!("## Archivo: {}\n\n```{lang}\n{safe}{trailing}```\n\n", rel.display())
}

fn split_into_pages(header: String, sections: Vec<String>, max_lines: usize) -> Vec<String> {
    let mut pages: Vec<String> = Vec::new();
    let mut current = header;
    let mut current_lines = current.lines().count();
    let mut page_has_section = false;

    for section in sections {
        let section_lines = section.lines().count();

        if page_has_section && current_lines + section_lines > max_lines {
            pages.push(current);
            let part = pages.len() + 1;
            current = format!(
                "# Contexto del proyecto (parte {part})\n\n\
                 > Continuación — ver parte 1 para el árbol de archivos.\n\n\
                 ---\n\n## Contenido de archivos\n\n"
            );
            current_lines = current.lines().count();
        }

        current_lines += section_lines;
        current.push_str(&section);
        page_has_section = true;
    }

    if page_has_section || pages.is_empty() {
        pages.push(current);
    }

    pages
}

fn main() -> io::Result<()> {
    let root = std::env::current_dir()?;
    let files = collect_files(&root);

    if files.is_empty() {
        eprintln!("No se encontraron archivos de texto en {}", root.display());
        return Ok(());
    }

    println!("Procesando {} archivo(s)…", files.len());

    let header = build_header(&root, &files);
    let sections: Vec<String> = files
        .iter()
        .map(|p| build_file_section(p, &root))
        .filter(|s| !s.is_empty())
        .collect();

    let pages = split_into_pages(header, sections, MAX_LINES_PER_FILE);

    if pages.len() == 1 {
        let output_path = root.join(OUTPUT_MD);
        fs::write(&output_path, pages[0].as_bytes())?;

        // Clean up zip from a previous run if it exists.
        let zip_path = root.join(OUTPUT_ZIP);
        if zip_path.exists() {
            fs::remove_file(&zip_path)?;
        }

        println!(
            "✓ {} generado ({} bytes)",
            OUTPUT_MD,
            pages[0].len()
        );
    } else {
        let zip_path = root.join(OUTPUT_ZIP);
        let file = fs::File::create(&zip_path)?;
        let mut zip = ZipWriter::new(file);
        let options = FileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);

        for (i, page) in pages.iter().enumerate() {
            let name = format!("context-{}.md", i + 1);
            zip.start_file(&name, options)?;
            zip.write_all(page.as_bytes())?;
        }

        let zip_file = zip.finish()?;
        let zip_size = zip_file.metadata().map(|m| m.len()).unwrap_or(0);

        // Clean up single context.md from a previous run if it exists.
        let md_path = root.join(OUTPUT_MD);
        if md_path.exists() {
            fs::remove_file(&md_path)?;
        }

        println!(
            "✓ {} generado — {} partes, {} bytes comprimidos",
            OUTPUT_ZIP,
            pages.len(),
            zip_size
        );
    }

    Ok(())
}
