use anyhow::Result;
use std::fs;
use std::path::Path;

use crate::db::Database;

pub fn run(path: &Path) -> Result<()> {
    if path.join("grove.db").exists() {
        println!("Grove already initialized at {}", path.display());
        return Ok(());
    }

    // Create directory structure
    fs::create_dir_all(path.join("tree"))?;
    fs::create_dir_all(path.join("drafts"))?;
    fs::create_dir_all(path.join("archive"))?;

    // Initialize database
    let _db = Database::create(path)?;

    // Initialize git repo for versioning
    init_git(path)?;

    println!("Grove planted at {}", path.display());
    println!();
    println!("  tree/     — your skill tree lives here");
    println!("  drafts/   — skills being evolved, not yet promoted");
    println!("  archive/  — pruned skills");
    println!("  grove.db  — metadata, usage tracking, evolution history");
    println!();
    println!("Next: grove grow <path> -d \"when to use this skill\"");

    Ok(())
}

fn init_git(path: &Path) -> Result<()> {
    // Only init git if not already in a git repo
    if path.join(".git").exists() {
        return Ok(());
    }

    // Try to init git, but don't fail if git isn't available
    let status = std::process::Command::new("git")
        .args(["init", "--quiet"])
        .current_dir(path)
        .status();

    match status {
        Ok(s) if s.success() => {
            // Write .gitignore
            fs::write(
                path.join(".gitignore"),
                "grove.db\ngrove.db-journal\ngrove.db-wal\ngrove.db-shm\n",
            )?;

            // Initial commit
            let _ = std::process::Command::new("git")
                .args(["add", "."])
                .current_dir(path)
                .status();
            let _ = std::process::Command::new("git")
                .args(["commit", "-m", "grove: plant initial grove", "--quiet"])
                .current_dir(path)
                .status();

            println!("  .git/     — version control for skill evolution");
        }
        _ => {
            eprintln!("  (git not available — skill versioning will use db only)");
        }
    }

    Ok(())
}
