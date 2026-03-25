use anyhow::{bail, Result};
use std::fs;
use std::path::Path;
use sha2::{Digest, Sha256};
use walkdir::WalkDir;

use crate::db::Database;
use crate::tree::git_commit;

/// Merge external skills into the grove tree
///
/// Supports:
/// - Claude Code plugin directories (with skills/ or .claude-plugin/)
/// - Flat directories of SKILL.md files
/// - Single SKILL.md files
pub fn run(
    db: &Database,
    grove_path: &Path,
    source: &Path,
    into: Option<&str>,
) -> Result<()> {
    if !source.exists() {
        bail!("source not found: {}", source.display());
    }

    let prefix = into.unwrap_or("");
    let mut merged = 0;
    let mut skipped = 0;

    if source.is_file() {
        // Single file merge
        if let Some(name) = source.file_stem().and_then(|s| s.to_str()) {
            let skill_name = sanitize_name(name);
            let skill_path = if prefix.is_empty() {
                skill_name.clone()
            } else {
                format!("{prefix}/{skill_name}")
            };

            match merge_skill_file(db, grove_path, source, &skill_path, &skill_name) {
                Ok(true) => merged += 1,
                Ok(false) => skipped += 1,
                Err(e) => eprintln!("  error merging {}: {e}", source.display()),
            }
        }
    } else if source.is_dir() {
        // Directory merge — detect structure
        let skills_dir = if source.join("skills").is_dir() {
            // Claude Code plugin structure: plugin-root/skills/
            source.join("skills")
        } else {
            source.to_path_buf()
        };

        // Walk the skills directory
        for entry in WalkDir::new(&skills_dir)
            .min_depth(1)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();

            // Look for SKILL.md files
            if path.is_file() {
                let filename = path.file_name().and_then(|f| f.to_str()).unwrap_or("");
                if filename == "SKILL.md" || filename.ends_with(".md") {
                    // Derive skill path from directory structure
                    let relative = path
                        .parent()
                        .unwrap_or(path)
                        .strip_prefix(&skills_dir)
                        .unwrap_or(Path::new(""));

                    let skill_path_str = if filename == "SKILL.md" {
                        // Directory-based skill: path comes from directories
                        let rel = relative.to_string_lossy().replace('\\', "/");
                        if rel.is_empty() {
                            // SKILL.md at root of skills dir
                            let parent_name = skills_dir
                                .file_name()
                                .and_then(|f| f.to_str())
                                .unwrap_or("imported");
                            parent_name.to_string()
                        } else {
                            rel
                        }
                    } else {
                        // File-based skill: name.md -> name
                        let name = path
                            .file_stem()
                            .and_then(|s| s.to_str())
                            .unwrap_or("imported");
                        let rel = relative.to_string_lossy().replace('\\', "/");
                        if rel.is_empty() {
                            name.to_string()
                        } else {
                            format!("{rel}/{name}")
                        }
                    };

                    let full_path = if prefix.is_empty() {
                        sanitize_path(&skill_path_str)
                    } else {
                        format!("{prefix}/{}", sanitize_path(&skill_path_str))
                    };

                    let name = full_path
                        .split('/')
                        .last()
                        .unwrap_or(&full_path)
                        .to_string();

                    match merge_skill_file(db, grove_path, path, &full_path, &name) {
                        Ok(true) => merged += 1,
                        Ok(false) => skipped += 1,
                        Err(e) => eprintln!("  error merging {}: {e}", path.display()),
                    }
                }
            }
        }
    }

    if merged > 0 {
        git_commit(grove_path, &format!("grove: merge {} skill(s) from {}", merged, source.display()));
    }

    println!();
    println!("Merged {} skill(s), skipped {} (already exist)", merged, skipped);

    Ok(())
}

fn merge_skill_file(
    db: &Database,
    grove_path: &Path,
    source_file: &Path,
    skill_path: &str,
    skill_name: &str,
) -> Result<bool> {
    // Check if skill already exists
    if db.get_skill_by_path(skill_path)?.is_some() {
        println!("  skip: {skill_path} (already exists)");
        return Ok(false);
    }

    // Read source content
    let content = fs::read_to_string(source_file)?;

    // Extract description from frontmatter if present
    let description = extract_description(&content).unwrap_or_default();

    // Create skill directory in grove tree
    let target_dir = grove_path.join("tree").join(skill_path);
    fs::create_dir_all(&target_dir)?;
    fs::write(target_dir.join("SKILL.md"), &content)?;

    // Compute hash
    let hash = content_hash(&content);

    // Find parent ID
    let parent_id = if let Some(idx) = skill_path.rfind('/') {
        let parent_path = &skill_path[..idx];
        db.get_skill_by_path(parent_path)?.map(|s| s.id)
    } else {
        None
    };

    // Insert into database
    let id = db.insert_skill(
        skill_name,
        skill_path,
        &description,
        parent_id.as_deref(),
        "merged",
        &hash,
    )?;

    println!("  merge: {skill_path} ({})", &id[..8]);
    Ok(true)
}

fn extract_description(content: &str) -> Option<String> {
    // Parse YAML frontmatter for description
    if !content.starts_with("---") {
        return None;
    }
    let rest = &content[3..];
    let end = rest.find("---")?;
    let frontmatter = &rest[..end];

    for line in frontmatter.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("description:") {
            let desc = trimmed.strip_prefix("description:")?.trim();
            // Handle quoted strings
            let desc = desc.trim_matches('"').trim_matches('\'');
            return Some(desc.to_string());
        }
    }
    None
}

fn content_hash(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    format!("{:x}", hasher.finalize())
}

fn sanitize_name(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '-'
            }
        })
        .collect::<String>()
        .to_lowercase()
}

fn sanitize_path(path: &str) -> String {
    path.split('/')
        .map(|seg| sanitize_name(seg))
        .collect::<Vec<_>>()
        .join("/")
}
