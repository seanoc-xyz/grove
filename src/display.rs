use anyhow::Result;
use std::collections::HashMap;
use std::path::Path;

use crate::db::{Database, Skill};

/// Display the skill tree as an ASCII tree
pub fn tree(db: &Database, _grove_path: &Path, max_depth: Option<usize>, show_stats: bool) -> Result<()> {
    let skills = db.list_skills()?;

    if skills.is_empty() {
        println!("(empty grove — run `grove grow <path>` to plant skills)");
        return Ok(());
    }

    // Build tree structure from paths
    let mut root_nodes: Vec<TreeNode> = Vec::new();
    let mut node_map: HashMap<String, Vec<TreeNode>> = HashMap::new();

    // Collect all unique path segments
    for skill in &skills {
        let segments: Vec<&str> = skill.path.split('/').collect();
        let depth = segments.len();

        let node = TreeNode {
            name: segments.last().unwrap().to_string(),
            path: skill.path.clone(),
            depth,
            skill: Some(skill.clone()),
        };

        if depth == 1 {
            root_nodes.push(node);
        } else {
            let parent_path = segments[..segments.len() - 1].join("/");
            node_map.entry(parent_path).or_default().push(node);
        }
    }

    // Create virtual nodes for intermediate paths that don't have corresponding skills
    let all_paths: Vec<String> = skills.iter().map(|s| s.path.clone()).collect();
    let mut intermediate_paths: Vec<String> = Vec::new();
    for path in &all_paths {
        let segments: Vec<&str> = path.split('/').collect();
        for i in 1..segments.len() {
            let intermediate = segments[..i].join("/");
            if !all_paths.contains(&intermediate) && !intermediate_paths.contains(&intermediate) {
                intermediate_paths.push(intermediate);
            }
        }
    }

    // Sort intermediate paths by depth (shallowest first) so parents exist before children
    intermediate_paths.sort_by_key(|p| p.matches('/').count());

    for ipath in &intermediate_paths {
        let segments: Vec<&str> = ipath.split('/').collect();
        let depth = segments.len();
        let node = TreeNode {
            name: segments.last().unwrap().to_string(),
            path: ipath.clone(),
            depth,
            skill: None,
        };

        if depth == 1 {
            root_nodes.push(node);
        } else {
            let parent_path = segments[..segments.len() - 1].join("/");
            node_map.entry(parent_path).or_default().push(node);
        }
    }

    // Sort roots
    root_nodes.sort_by(|a, b| a.name.cmp(&b.name));

    println!("grove");
    for (i, node) in root_nodes.iter().enumerate() {
        let is_last = i == root_nodes.len() - 1;
        print_node(node, "", is_last, &node_map, show_stats, max_depth, 1);
    }

    Ok(())
}

struct TreeNode {
    name: String,
    path: String,
    #[allow(dead_code)]
    depth: usize,
    skill: Option<Skill>,
}

fn print_node(
    node: &TreeNode,
    prefix: &str,
    is_last: bool,
    children_map: &HashMap<String, Vec<TreeNode>>,
    show_stats: bool,
    max_depth: Option<usize>,
    current_depth: usize,
) {
    let connector = if is_last { "└── " } else { "├── " };
    let stats_str = if show_stats {
        if let Some(skill) = &node.skill {
            if skill.usage_count > 0 {
                let rate = (skill.success_count as f64 / skill.usage_count as f64) * 100.0;
                format!(" (v{}, {}x, {:.0}%)", skill.version, skill.usage_count, rate)
            } else {
                format!(" (v{})", skill.version)
            }
        } else {
            String::new()
        }
    } else {
        String::new()
    };

    println!("{prefix}{connector}{}{stats_str}", node.name);

    if let Some(depth) = max_depth {
        if current_depth >= depth {
            return;
        }
    }

    let child_prefix = format!("{}{}", prefix, if is_last { "    " } else { "│   " });

    if let Some(children) = children_map.get(&node.path) {
        let mut sorted: Vec<&TreeNode> = children.iter().collect();
        sorted.sort_by(|a, b| a.name.cmp(&b.name));
        for (i, child) in sorted.iter().enumerate() {
            let child_is_last = i == sorted.len() - 1;
            print_node(child, &child_prefix, child_is_last, children_map, show_stats, max_depth, current_depth + 1);
        }
    }
}

/// List all skills
pub fn list(db: &Database, flat: bool) -> Result<()> {
    let skills = db.list_skills()?;

    if skills.is_empty() {
        println!("(empty grove — run `grove grow <path>` to plant skills)");
        return Ok(());
    }

    if flat {
        for skill in &skills {
            println!("{:<30} v{:<3} {:>4}x  {}",
                skill.path, skill.version, skill.usage_count,
                if skill.description.is_empty() { "(no description)" } else { &skill.description });
        }
    } else {
        println!("{:<30} {:>5} {:>6} {:>5}  {}",
            "PATH", "VER", "USAGE", "RATE", "DESCRIPTION");
        println!("{}", "-".repeat(80));
        for skill in &skills {
            let rate = if skill.usage_count > 0 {
                format!("{:.0}%", (skill.success_count as f64 / skill.usage_count as f64) * 100.0)
            } else {
                "-".to_string()
            };
            let desc = if skill.description.len() > 30 {
                format!("{}...", &skill.description[..27])
            } else if skill.description.is_empty() {
                "(no description)".to_string()
            } else {
                skill.description.clone()
            };
            println!("{:<30} {:>5} {:>6} {:>5}  {}",
                skill.path,
                format!("v{}", skill.version),
                skill.usage_count,
                rate,
                desc);
        }
    }

    println!();
    println!("{} skill(s)", skills.len());

    Ok(())
}
