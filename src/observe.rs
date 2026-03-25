use anyhow::{bail, Result};

use crate::db::Database;
use crate::error::GroveError;

/// Record a usage observation for a skill
pub fn record(
    db: &Database,
    skill_path: &str,
    outcome: &str,
    context: Option<&str>,
    suggestion: Option<&str>,
) -> Result<()> {
    // Validate outcome
    match outcome {
        "success" | "failure" | "partial" => {}
        _ => bail!(GroveError::InvalidOutcome(outcome.to_string())),
    }

    let skill = db
        .get_skill_by_path(skill_path)?
        .ok_or_else(|| GroveError::SkillNotFound(skill_path.to_string()))?;

    let obs_id = db.insert_observation(&skill.id, outcome, context, suggestion)?;

    println!("Recorded [{outcome}] for {skill_path}");
    println!("  observation: {}", &obs_id[..8]);
    println!("  total usage: {}", skill.usage_count + 1);

    if outcome == "failure" {
        let obs = db.get_observations(&skill.id, true)?;
        let failures: Vec<_> = obs.iter().filter(|o| o.outcome == "failure").collect();
        if failures.len() >= 3 {
            println!();
            println!("  ! {} failures accumulated — consider running `grove evolve {}`",
                failures.len(), skill_path);
        }
    }

    Ok(())
}

/// Show health dashboard
pub fn health(db: &Database, subtree: Option<&str>) -> Result<()> {
    let skills = db.list_skills()?;

    let filtered: Vec<_> = if let Some(prefix) = subtree {
        skills.into_iter().filter(|s| s.path.starts_with(prefix)).collect()
    } else {
        skills
    };

    if filtered.is_empty() {
        println!("No skills found.");
        return Ok(());
    }

    println!("--- Grove Health ---");
    println!();

    // Overall stats
    let total_usage: u64 = filtered.iter().map(|s| s.usage_count).sum();
    let total_success: u64 = filtered.iter().map(|s| s.success_count).sum();
    let total_failure: u64 = filtered.iter().map(|s| s.failure_count).sum();
    let total_skills = filtered.len();

    println!("  Skills:   {total_skills}");
    println!("  Usage:    {total_usage}");
    if total_usage > 0 {
        let rate = (total_success as f64 / total_usage as f64) * 100.0;
        println!("  Success:  {total_success} ({rate:.0}%)");
        println!("  Failure:  {total_failure}");
    }
    println!();

    // Most used
    let mut by_usage = filtered.clone();
    by_usage.sort_by(|a, b| b.usage_count.cmp(&a.usage_count));
    let top: Vec<_> = by_usage.iter().filter(|s| s.usage_count > 0).take(5).collect();

    if !top.is_empty() {
        println!("  Most used:");
        for s in &top {
            let rate = if s.usage_count > 0 {
                format!("{:.0}%", (s.success_count as f64 / s.usage_count as f64) * 100.0)
            } else {
                "-".to_string()
            };
            println!("    {:<25} {}x  ({} success)", s.path, s.usage_count, rate);
        }
        println!();
    }

    // Struggling (high failure rate)
    let mut struggling: Vec<_> = filtered.iter()
        .filter(|s| s.usage_count > 0 && s.failure_count > 0)
        .collect();
    struggling.sort_by(|a, b| {
        let rate_a = a.failure_count as f64 / a.usage_count as f64;
        let rate_b = b.failure_count as f64 / b.usage_count as f64;
        rate_b.partial_cmp(&rate_a).unwrap_or(std::cmp::Ordering::Equal)
    });

    if !struggling.is_empty() {
        println!("  Needs evolution (high failure rate):");
        for s in struggling.iter().take(5) {
            let rate = (s.failure_count as f64 / s.usage_count as f64) * 100.0;
            println!("    {:<25} {:.0}% failure ({}/{})",
                s.path, rate, s.failure_count, s.usage_count);
        }
        println!();
    }

    // Never used
    let unused: Vec<_> = filtered.iter().filter(|s| s.usage_count == 0).collect();
    if !unused.is_empty() {
        println!("  Never used ({}):", unused.len());
        for s in unused.iter().take(10) {
            println!("    {}", s.path);
        }
        if unused.len() > 10 {
            println!("    ... and {} more", unused.len() - 10);
        }
        println!();
    }

    // Pending evolution (unconsumed observations)
    let mut pending_evolution = Vec::new();
    for s in &filtered {
        let obs = db.get_observations(&s.id, true)?;
        if !obs.is_empty() {
            pending_evolution.push((s, obs.len()));
        }
    }
    if !pending_evolution.is_empty() {
        println!("  Pending evolution:");
        for (s, count) in &pending_evolution {
            println!("    {:<25} {} observation(s) to process", s.path, count);
        }
        println!();
    }

    Ok(())
}

/// Overall grove statistics
pub fn stats(db: &Database) -> Result<()> {
    let skill_count = db.skill_count()?;
    let obs_count = db.observation_count()?;
    let total_usage = db.total_usage()?;
    let total_success = db.total_successes()?;
    let total_failure = db.total_failures()?;

    println!("--- Grove Stats ---");
    println!();
    println!("  Skills:       {skill_count}");
    println!("  Observations: {obs_count}");
    println!("  Total usage:  {total_usage}");
    if total_usage > 0 {
        let rate = (total_success as f64 / total_usage as f64) * 100.0;
        println!("  Success rate: {rate:.0}% ({total_success}/{total_usage})");
        println!("  Failures:     {total_failure}");
    }

    println!();

    // Top skills
    let top = db.top_skills(5)?;
    if !top.is_empty() {
        println!("  Top skills by usage:");
        for s in &top {
            if s.usage_count > 0 {
                println!("    {:<25} {}x (v{})", s.path, s.usage_count, s.version);
            }
        }
        println!();
    }

    // Skills needing attention
    let struggling = db.struggling_skills(3)?;
    let needing: Vec<_> = struggling.iter().filter(|s| s.failure_count > 0).collect();
    if !needing.is_empty() {
        println!("  Needs attention:");
        for s in &needing {
            let rate = (s.failure_count as f64 / s.usage_count as f64) * 100.0;
            println!("    {:<25} {:.0}% failure rate", s.path, rate);
        }
    }

    Ok(())
}
