use anyhow::{bail, Result};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::Path;

use crate::db::Database;
use crate::error::GroveError;

/// Grow a new skill at the given tree path
pub fn grow(
    db: &Database,
    grove_path: &Path,
    skill_path: &str,
    description: Option<&str>,
    from: Option<&str>,
) -> Result<()> {
    validate_path(skill_path)?;

    // Check if skill already exists
    if db.get_skill_by_path(skill_path)?.is_some() {
        bail!(GroveError::SkillExists(skill_path.to_string()));
    }

    // Derive name from last segment
    let name = skill_path
        .split('/')
        .last()
        .unwrap_or(skill_path);

    let desc = description.unwrap_or("");

    // Create SKILL.md content
    let content = if let Some(template_path) = from {
        // Copy from existing skill
        let template = db.get_skill_by_path(template_path)?;
        match template {
            Some(_) => {
                let template_file = grove_path.join("tree").join(template_path).join("SKILL.md");
                if template_file.exists() {
                    fs::read_to_string(&template_file)?
                } else {
                    default_skill_content(name, desc)
                }
            }
            None => {
                // Maybe it's a file path
                let p = Path::new(template_path);
                if p.exists() {
                    fs::read_to_string(p)?
                } else {
                    bail!(GroveError::SkillNotFound(template_path.to_string()));
                }
            }
        }
    } else {
        default_skill_content(name, desc)
    };

    // Create the skill directory and SKILL.md
    let skill_dir = grove_path.join("tree").join(skill_path);
    fs::create_dir_all(&skill_dir)?;
    let skill_file = skill_dir.join("SKILL.md");
    fs::write(&skill_file, &content)?;

    // Compute content hash
    let hash = content_hash(&content);

    // Find parent
    let parent_id = find_parent_id(db, skill_path)?;

    // Insert into database
    let id = db.insert_skill(name, skill_path, desc, parent_id.as_deref(), "native", &hash)?;

    // Git commit
    git_commit(grove_path, &format!("grove: grow {skill_path}"));

    println!("Grew skill: {skill_path}");
    println!("  id: {id}");
    println!("  version: 1");
    if let Some(pid) = &parent_id {
        println!("  parent: {pid}");
    }
    println!("  file: {}", skill_file.display());

    Ok(())
}

/// Show a skill's content and metadata
pub fn show(db: &Database, grove_path: &Path, skill_path: &str) -> Result<()> {
    let skill = db
        .get_skill_by_path(skill_path)?
        .ok_or_else(|| GroveError::SkillNotFound(skill_path.to_string()))?;

    println!("--- {} ---", skill.path);
    println!("  name:        {}", skill.name);
    println!("  version:     {}", skill.version);
    println!("  source:      {}", skill.source);
    println!("  description: {}", skill.description);
    println!("  usage:       {} total, {} success, {} failure",
        skill.usage_count, skill.success_count, skill.failure_count);
    if skill.usage_count > 0 {
        let rate = (skill.success_count as f64 / skill.usage_count as f64) * 100.0;
        println!("  success rate: {:.0}%", rate);
    }
    println!("  created:     {}", skill.created_at);
    if let Some(evolved) = &skill.evolved_at {
        println!("  evolved:     {}", evolved);
    }
    println!("  id:          {}", skill.id);
    println!("  hash:        {}", &skill.content_hash[..12]);

    // Show content
    let skill_file = grove_path.join("tree").join(skill_path).join("SKILL.md");
    if skill_file.exists() {
        println!();
        let content = fs::read_to_string(&skill_file)?;
        println!("{}", content);
    }

    // Show unconsumed observations
    let obs = db.get_observations(&skill.id, true)?;
    if !obs.is_empty() {
        println!();
        println!("--- Pending observations ({}) ---", obs.len());
        for o in &obs {
            println!("  [{}] {} — {}",
                o.outcome,
                o.created_at.split('T').next().unwrap_or(&o.created_at),
                o.context.as_deref().unwrap_or("(no context)"));
            if let Some(s) = &o.suggestion {
                println!("    suggestion: {s}");
            }
        }
    }

    Ok(())
}

/// Prune a skill from the tree
pub fn prune(db: &Database, grove_path: &Path, skill_path: &str, archive: bool) -> Result<()> {
    let _skill = db
        .get_skill_by_path(skill_path)?
        .ok_or_else(|| GroveError::SkillNotFound(skill_path.to_string()))?;

    let skill_dir = grove_path.join("tree").join(skill_path);

    if archive {
        // Move to archive
        let archive_dir = grove_path.join("archive").join(skill_path);
        if let Some(parent) = archive_dir.parent() {
            fs::create_dir_all(parent)?;
        }
        if skill_dir.exists() {
            fs::rename(&skill_dir, &archive_dir)?;
        }
        db.archive_skill(skill_path)?;
        println!("Archived skill: {skill_path} -> archive/{skill_path}");
    } else {
        // Delete
        if skill_dir.exists() {
            fs::remove_dir_all(&skill_dir)?;
        }
        db.delete_skill(skill_path)?;
        println!("Pruned skill: {skill_path}");
    }

    git_commit(grove_path, &format!("grove: prune {skill_path}"));
    Ok(())
}

/// Show version history of a skill
pub fn history(db: &Database, skill_path: &str) -> Result<()> {
    let skill = db
        .get_skill_by_path(skill_path)?
        .ok_or_else(|| GroveError::SkillNotFound(skill_path.to_string()))?;

    let versions = db.get_versions(&skill.id)?;

    println!("--- History: {} ---", skill_path);
    println!("  current version: {}", skill.version);
    println!();

    for v in &versions {
        let marker = if v.version == skill.version { " <- current" } else { "" };
        println!("  v{} [{}] {}{}", v.version, &v.content_hash[..8], v.description, marker);
        println!("    {}", v.created_at);
    }

    if versions.is_empty() {
        println!("  (no version history)");
    }

    Ok(())
}

/// Trigger evolution of a skill based on unconsumed observations
pub fn evolve(db: &Database, grove_path: &Path, skill_path: &str, dry_run: bool) -> Result<()> {
    let skill = db
        .get_skill_by_path(skill_path)?
        .ok_or_else(|| GroveError::SkillNotFound(skill_path.to_string()))?;

    let observations = db.get_observations(&skill.id, true)?;

    if observations.is_empty() {
        println!("No unconsumed observations for {skill_path}. Nothing to evolve.");
        return Ok(());
    }

    println!("--- Evolve: {} ---", skill_path);
    println!("  {} unconsumed observations:", observations.len());
    println!();

    let mut failures = Vec::new();
    let mut suggestions = Vec::new();

    for o in &observations {
        println!("  [{}] {}", o.outcome, o.context.as_deref().unwrap_or("(no context)"));
        if o.outcome == "failure" {
            if let Some(ctx) = &o.context {
                failures.push(ctx.clone());
            }
        }
        if let Some(s) = &o.suggestion {
            suggestions.push(s.clone());
            println!("    -> {s}");
        }
    }

    if dry_run {
        println!();
        println!("(dry run — no changes applied)");
        println!();
        println!("Recommended actions:");
        if !failures.is_empty() {
            println!("  - Add gotchas section addressing {} failure(s)", failures.len());
        }
        if !suggestions.is_empty() {
            println!("  - Incorporate {} suggestion(s) into SKILL.md", suggestions.len());
        }
        println!("  - Run without --dry-run to apply evolution");
        return Ok(());
    }

    // Read current content
    let skill_file = grove_path.join("tree").join(skill_path).join("SKILL.md");
    let mut content = if skill_file.exists() {
        fs::read_to_string(&skill_file)?
    } else {
        String::new()
    };

    // Append gotchas from failures
    if !failures.is_empty() || !suggestions.is_empty() {
        content.push_str("\n\n## Gotchas (auto-evolved)\n\n");
        for f in &failures {
            content.push_str(&format!("- FAILURE: {f}\n"));
        }
        for s in &suggestions {
            content.push_str(&format!("- SUGGESTION: {s}\n"));
        }
    }

    // Write updated content
    fs::write(&skill_file, &content)?;
    let hash = content_hash(&content);

    // Update database
    let desc = format!("evolved from {} observations", observations.len());
    db.update_skill_content(skill_path, &hash, &desc)?;
    db.mark_observations_consumed(&skill.id)?;

    // Git commit
    git_commit(grove_path, &format!("grove: evolve {skill_path} (v{})", skill.version + 1));

    println!();
    println!("Evolved {skill_path} to v{}", skill.version + 1);
    println!("  processed {} observation(s)", observations.len());
    println!("  {} failure(s) added as gotchas", failures.len());
    println!("  {} suggestion(s) incorporated", suggestions.len());

    Ok(())
}

/// Generate a Claude Code plugin manifest from the grove tree
pub fn sync_plugin(db: &Database, grove_path: &Path, output: Option<&str>) -> Result<()> {
    let skills = db.list_skills()?;

    let output_dir = output
        .map(|p| Path::new(p).to_path_buf())
        .unwrap_or_else(|| grove_path.to_path_buf());

    // Create .claude-plugin directory
    let plugin_dir = output_dir.join(".claude-plugin");
    fs::create_dir_all(&plugin_dir)?;

    // Generate plugin.json
    let plugin_json = serde_json::json!({
        "name": "grove-skills",
        "description": "Skills managed by Grove — the self-evolving skill tree",
        "version": "0.1.0"
    });
    fs::write(
        plugin_dir.join("plugin.json"),
        serde_json::to_string_pretty(&plugin_json)?,
    )?;

    // Create skills directory with symlinks or copies
    let skills_dir = output_dir.join("skills");
    fs::create_dir_all(&skills_dir)?;

    let mut synced = 0;
    for skill in &skills {
        let source = grove_path.join("tree").join(&skill.path).join("SKILL.md");
        if source.exists() {
            let target_dir = skills_dir.join(&skill.path);
            if let Some(parent) = target_dir.parent() {
                fs::create_dir_all(parent)?;
            }
            // Copy the SKILL.md
            let target = if skill.path.contains('/') {
                let name = skill.path.replace('/', "-");
                skills_dir.join(format!("{name}.md"))
            } else {
                skills_dir.join(format!("{}.md", skill.name))
            };

            // Read source and write with proper frontmatter for Claude Code
            let content = fs::read_to_string(&source)?;
            fs::write(&target, &content)?;
            synced += 1;
        }
    }

    println!("Synced {} skill(s) to {}", synced, output_dir.display());
    println!("  plugin: {}", plugin_dir.display());
    println!("  skills: {}", skills_dir.display());
    println!();
    println!("To use: claude --plugin-dir {}", output_dir.display());

    Ok(())
}

// --- Helpers ---

fn validate_path(path: &str) -> Result<()> {
    if path.is_empty() {
        bail!(GroveError::InvalidPath(path.to_string()));
    }
    // Allow alphanumeric, hyphens, underscores, slashes
    for ch in path.chars() {
        if !ch.is_alphanumeric() && ch != '/' && ch != '-' && ch != '_' {
            bail!(GroveError::InvalidPath(path.to_string()));
        }
    }
    // No leading/trailing/double slashes
    if path.starts_with('/') || path.ends_with('/') || path.contains("//") {
        bail!(GroveError::InvalidPath(path.to_string()));
    }
    Ok(())
}

fn content_hash(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    format!("{:x}", hasher.finalize())
}

fn find_parent_id(db: &Database, skill_path: &str) -> Result<Option<String>> {
    if let Some(idx) = skill_path.rfind('/') {
        let parent_path = &skill_path[..idx];
        if let Some(parent) = db.get_skill_by_path(parent_path)? {
            return Ok(Some(parent.id));
        }
    }
    Ok(None)
}

fn default_skill_content(name: &str, description: &str) -> String {
    let mut content = String::new();
    content.push_str("---\n");
    content.push_str(&format!("name: {name}\n"));
    content.push_str(&format!("description: {description}\n"));
    content.push_str("---\n\n");
    content.push_str(&format!("# {name}\n\n"));
    if !description.is_empty() {
        content.push_str(&format!("{description}\n\n"));
    }
    content.push_str("## When to use\n\n");
    content.push_str("<!-- Describe when this skill should be triggered -->\n\n");
    content.push_str("## Instructions\n\n");
    content.push_str("<!-- Step-by-step instructions for the agent -->\n\n");
    content.push_str("## Gotchas\n\n");
    content.push_str("<!-- Accumulated failure patterns and edge cases -->\n");
    content
}

pub fn git_commit(grove_path: &Path, message: &str) {
    let _ = std::process::Command::new("git")
        .args(["add", "."])
        .current_dir(grove_path)
        .status();
    let _ = std::process::Command::new("git")
        .args(["commit", "-m", message, "--quiet", "--allow-empty"])
        .current_dir(grove_path)
        .status();
}
