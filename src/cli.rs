use clap::{Parser, Subcommand};
use std::path::PathBuf;

/// Grove — the self-evolving skill tree for AI agents
#[derive(Parser)]
#[command(name = "grove", version, about)]
pub struct Cli {
    /// Path to the grove (default: ~/.grove)
    #[arg(long, global = true)]
    pub grove: Option<PathBuf>,

    #[command(subcommand)]
    pub command: Command,
}

impl Cli {
    pub fn grove_path(&self) -> PathBuf {
        self.grove
            .clone()
            .or_else(|| std::env::var("GROVE_PATH").ok().map(PathBuf::from))
            .unwrap_or_else(|| {
                dirs_fallback().join(".grove")
            })
    }
}

fn dirs_fallback() -> PathBuf {
    std::env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."))
}

#[derive(Subcommand)]
pub enum Command {
    /// Plant a new grove (initialize skill tree)
    Init {
        /// Directory to initialize (default: ~/.grove)
        #[arg()]
        path: Option<PathBuf>,
    },

    /// Visualize the skill tree
    Tree {
        /// Maximum depth to display
        #[arg(long)]
        depth: Option<usize>,
        /// Show usage statistics alongside tree
        #[arg(long)]
        stats: bool,
    },

    /// Grow a new skill at a position in the tree
    Grow {
        /// Tree path for the skill (e.g., "coding/review/security")
        #[arg()]
        path: String,
        /// Skill description (trigger logic — when to use)
        #[arg(long, short)]
        description: Option<String>,
        /// Template or existing skill to grow from
        #[arg(long)]
        from: Option<String>,
    },

    /// Display a skill's content and metadata
    Show {
        /// Tree path of the skill
        #[arg()]
        path: String,
    },

    /// List all skills
    List {
        /// Flat list instead of tree
        #[arg(long)]
        flat: bool,
    },

    /// Record a usage observation for a skill
    Observe {
        /// Tree path of the skill
        #[arg()]
        path: String,
        /// Outcome: success, failure, or partial
        #[arg()]
        outcome: String,
        /// What happened (context)
        #[arg(long, short)]
        context: Option<String>,
        /// Proposed improvement
        #[arg(long, short)]
        suggestion: Option<String>,
    },

    /// Merge external skills into the grove tree
    Merge {
        /// Source directory (Claude Code plugin dir or git repo path)
        #[arg()]
        source: PathBuf,
        /// Target path in grove tree (default: root)
        #[arg(long)]
        into: Option<String>,
    },

    /// Usage and health dashboard
    Health {
        /// Subtree to inspect (default: all)
        #[arg()]
        path: Option<String>,
    },

    /// Overall grove statistics
    Stats,

    /// Prune (remove/archive) a skill
    Prune {
        /// Tree path of the skill
        #[arg()]
        path: String,
        /// Archive instead of delete
        #[arg(long)]
        archive: bool,
    },

    /// Show version history of a skill's evolution
    History {
        /// Tree path of the skill
        #[arg()]
        path: String,
    },

    /// Trigger evolution of a skill based on observations
    Evolve {
        /// Tree path of the skill
        #[arg()]
        path: String,
        /// Show what would change without applying
        #[arg(long)]
        dry_run: bool,
    },

    /// Generate Claude Code plugin manifest from grove tree
    Sync {
        /// Output directory for plugin files (default: grove dir)
        #[arg(long)]
        output: Option<String>,
    },
}
