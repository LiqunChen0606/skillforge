use std::fs;
use std::path::{Path, PathBuf};

use aif_parser::{detect_syntax_version, migrate::migrate_v1_to_v2, SyntaxVersion};

/// Handle the `aif migrate-syntax` CLI command.
pub fn handle_migrate_syntax(path: PathBuf, in_place: bool, dry_run: bool) {
    let files = if path.is_dir() {
        collect_aif_files(&path)
    } else {
        vec![path.clone()]
    };

    if files.is_empty() {
        eprintln!("No .aif files found at {}", path.display());
        std::process::exit(1);
    }

    let mut changed = 0usize;
    let mut skipped = 0usize;
    let mut errored = 0usize;

    for file in &files {
        match process_file(file, in_place, dry_run, files.len() > 1) {
            Ok(Outcome::Changed) => changed += 1,
            Ok(Outcome::Skipped) => skipped += 1,
            Err(e) => {
                eprintln!("{}: {}", file.display(), e);
                errored += 1;
            }
        }
    }

    if files.len() > 1 || dry_run {
        eprintln!(
            "\nmigrate-syntax: {} changed, {} skipped, {} errored",
            changed, skipped, errored
        );
    }

    if errored > 0 {
        std::process::exit(1);
    }
    // Exit 2 when nothing needed changes (useful for scripts).
    if changed == 0 && errored == 0 {
        std::process::exit(2);
    }
}

enum Outcome {
    Changed,
    Skipped,
}

fn process_file(
    file: &Path,
    in_place: bool,
    dry_run: bool,
    multi_file: bool,
) -> Result<Outcome, String> {
    let input = fs::read_to_string(file).map_err(|e| format!("read error: {}", e))?;

    match detect_syntax_version(&input) {
        Ok(SyntaxVersion::V2) => {
            if dry_run || multi_file {
                eprintln!("{}: already v2, skipping", file.display());
            }
            return Ok(Outcome::Skipped);
        }
        Ok(SyntaxVersion::V1) => {}
        Err(e) => return Err(e),
    }

    let migrated = migrate_v1_to_v2(&input);

    // Sanity: re-parse migrated output as v2 to confirm fidelity.
    if let Err(errs) = aif_parser::parse_with_version(&migrated, SyntaxVersion::V2) {
        return Err(format!(
            "post-migration parse failed: {}",
            errs.first().map(|e| e.message.clone()).unwrap_or_default()
        ));
    }

    if dry_run {
        eprintln!("{}: would migrate to v2", file.display());
        return Ok(Outcome::Changed);
    }

    if in_place {
        fs::write(file, &migrated).map_err(|e| format!("write error: {}", e))?;
        if multi_file {
            eprintln!("{}: migrated", file.display());
        }
    } else {
        print!("{}", migrated);
    }

    Ok(Outcome::Changed)
}

fn collect_aif_files(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    walk(dir, &mut out);
    out
}

fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            walk(&path, out);
        } else if path.extension().and_then(|s| s.to_str()) == Some("aif") {
            out.push(path);
        }
    }
}
