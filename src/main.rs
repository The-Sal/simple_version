mod objs;
use clap::{CommandFactory, Parser, Subcommand};
use objs::*;
use sha2::Digest;
use std::fs;
use std::io::Write;
use std::path::Path;

use syn::spanned::Spanned;
use syn::Item;

const VERSION_FILE: &str = "versionx.json";
const CHANGELOG_FILE: &str = "changelog.txt";

#[derive(Parser)]
#[command(name = "simple_version")]
#[command(about = "Track semantic versioning from code changes")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Path to scan (defaults to current directory)
    #[arg(default_value = ".")]
    path: String,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize versionx.json with current code state
    Init,
    /// Bump version based on code changes and update changelog
    Bump,
    /// Force a major version bump
    Major,
}

fn extract_symbols(file_path: &str) -> Vec<GenericSymbol> {
    let content = fs::read_to_string(file_path).expect("Unable to read file");
    let lines: Vec<&str> = content.lines().collect();

    let parsed = match syn::parse_file(&content) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Failed to parse {}: {}", file_path, e);
            return vec![];
        }
    };

    let mut symbols: Vec<GenericSymbol> = vec![];

    for item in parsed.items {
        if let Item::Fn(f) = &item {
            let symbol_name = f.sig.ident.to_string();
            let span = f.span();
            let start_line = span.start().line;
            let end_line = span.end().line;

            let extract = lines[(start_line - 1)..end_line].join("\n");
            let extract_hash = hex::encode(sha2::Sha256::digest(extract.as_bytes()));
            symbols.push(GenericSymbol {
                name: symbol_name,
                hash: extract_hash,
                type_of: ObjectType::Function,
            });
        }
        if let Item::Struct(s) = &item {
            let symbol_name = s.ident.to_string();
            let span = s.span();
            let start_line = span.start().line;
            let end_line = span.end().line;
            let extract = lines[(start_line - 1)..end_line].join("\n");
            let extract_hash = hex::encode(sha2::Sha256::digest(extract.as_bytes()));
            symbols.push(GenericSymbol {
                name: symbol_name,
                hash: extract_hash,
                type_of: ObjectType::Struct,
            });
        }
    }

    symbols
}

fn should_ignore(path: &Path, gitignore_patterns: &[String]) -> bool {
    let path_str = path.to_string_lossy();

    // Always ignore .git directory and target directory
    if path_str.contains("/.git/") || path_str.ends_with("/.git") {
        return true;
    }
    if path_str.contains("/target/") || path_str.ends_with("/target") {
        return true;
    }

    // Check against gitignore patterns
    for pattern in gitignore_patterns {
        let pattern = pattern.trim();
        if pattern.is_empty() || pattern.starts_with('#') {
            continue;
        }

        // Simple pattern matching (can be enhanced for full gitignore support)
        if pattern.ends_with('/') {
            // Directory pattern
            let dir_pattern = &pattern[..pattern.len() - 1];
            if path_str.contains(&format!("/{}/", dir_pattern))
                || path_str.ends_with(&format!("/{}", dir_pattern))
            {
                return true;
            }
        } else if path
            .file_name()
            .and_then(|n| n.to_str())
            .map(|n| n == pattern)
            .unwrap_or(false)
        {
            return true;
        } else if path_str.contains(&format!("/{}", pattern)) {
            return true;
        }
    }

    false
}

fn load_gitignore(path: &str) -> Vec<String> {
    let gitignore_path = Path::new(path).join(".gitignore");
    if !gitignore_path.exists() {
        return vec![];
    }

    fs::read_to_string(gitignore_path)
        .unwrap_or_default()
        .lines()
        .map(|s| s.to_string())
        .collect()
}

fn scan_directory_recursive(
    path: &Path,
    gitignore_patterns: &[String],
    symbols: &mut Vec<GenericSymbol>,
) {
    if should_ignore(path, gitignore_patterns) {
        return;
    }

    if path.is_file() && path.extension().and_then(|e| e.to_str()) == Some("rs") {
        let symbols_found = extract_symbols(path.to_str().unwrap());
        symbols.extend(symbols_found);
    } else if path.is_dir() {
        if let Ok(entries) = fs::read_dir(path) {
            for entry in entries.flatten() {
                scan_directory_recursive(&entry.path(), gitignore_patterns, symbols);
            }
        }
    }
}

fn scan_directory(path: &str) -> Vec<GenericSymbol> {
    let mut all_symbols = vec![];
    let gitignore_patterns = load_gitignore(path);
    let metadata = fs::metadata(path).expect("Unable to get metadata");

    if metadata.is_dir() {
        scan_directory_recursive(Path::new(path), &gitignore_patterns, &mut all_symbols);
    } else if metadata.is_file() && path.ends_with(".rs") {
        let symbols = extract_symbols(path);
        all_symbols.extend(symbols);
    }

    all_symbols
}

fn load_version_file() -> Option<SymbolTable> {
    if Path::new(VERSION_FILE).exists() {
        let content = fs::read_to_string(VERSION_FILE).ok()?;
        serde_json::from_str(&content).ok()
    } else {
        None
    }
}

fn save_version_file(table: &SymbolTable) {
    let json = serde_json::to_string_pretty(table).expect("Failed to serialize version file");
    fs::write(VERSION_FILE, json).expect("Failed to write version file");
}

fn append_changelog(changes: &Changes, version: &str) {
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(CHANGELOG_FILE)
        .expect("Failed to open changelog file");

    writeln!(file, "Version {}", version).expect("Failed to write to changelog");
    writeln!(file, "{}", changes.generate_change_log()).expect("Failed to write changes");
    writeln!(file, "{}", "=".repeat(100)).expect("Failed to write separator");
}

fn main() {
    let cli = Cli::parse();

    let command = match cli.command {
        Some(cmd) => cmd,
        None => {
            let mut cmd = Cli::command();
            cmd.print_help().unwrap();
            println!();
            std::process::exit(0);
        }
    };

    match command {
        Commands::Init => {
            if Path::new(VERSION_FILE).exists() {
                println!("{} already exists. Use 'bump' to update.", VERSION_FILE);
                return;
            }

            let symbols = scan_directory(&cli.path);
            let table = SymbolTable::new(symbols);

            save_version_file(&table);
            println!(
                "Initialized {} with version {}",
                VERSION_FILE,
                table.version_string()
            );
        }

        Commands::Bump => {
            let symbols = scan_directory(&cli.path);

            let mut table = match load_version_file() {
                Some(t) => t,
                None => {
                    println!("No {} found. Creating initial version...", VERSION_FILE);
                    let new_table = SymbolTable::new(symbols);
                    save_version_file(&new_table);
                    println!(
                        "Created {} with version {}",
                        VERSION_FILE,
                        new_table.version_string()
                    );
                    return;
                }
            };

            let old_version = table.version_string();
            let changes = table.compare_and_swap(symbols, false);

            if changes.has_changes() {
                let new_version = table.version_string();
                save_version_file(&table);
                append_changelog(&changes, &new_version);
                println!("Bumped version: {} -> {}", old_version, new_version);
                println!(
                    "Updated {} and appended to {}",
                    VERSION_FILE, CHANGELOG_FILE
                );
            } else {
                println!("No changes detected. Version remains {}", old_version);
            }
        }

        Commands::Major => {
            let symbols = scan_directory(&cli.path);

            let mut table = match load_version_file() {
                Some(t) => t,
                None => {
                    println!(
                        "No {} found. Creating initial version with major=1...",
                        VERSION_FILE
                    );
                    let mut new_table = SymbolTable::new(symbols);
                    new_table.major_version = 1;
                    new_table.minor_version = 0;
                    new_table.patch_version = 0;
                    save_version_file(&new_table);
                    println!("Created {} with version 1.0.0", VERSION_FILE);
                    return;
                }
            };

            let old_version = table.version_string();
            let changes = table.compare_and_swap(symbols, true);

            let new_version = table.version_string();
            save_version_file(&table);
            append_changelog(&changes, &new_version);
            println!("Forced major bump: {} -> {}", old_version, new_version);
            println!(
                "Updated {} and appended to {}",
                VERSION_FILE, CHANGELOG_FILE
            );
        }
    }
}
