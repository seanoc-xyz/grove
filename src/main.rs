mod cli;
mod db;
mod display;
mod error;
mod init;
mod merge;
mod observe;
mod tree;

use anyhow::Result;
use clap::Parser;
use cli::{Cli, Command};

fn main() -> Result<()> {
    let cli = Cli::parse();
    let grove_path = cli.grove_path();

    match cli.command {
        Command::Init { path } => {
            let p = path.unwrap_or_else(|| grove_path.clone());
            init::run(&p)
        }
        Command::Tree { depth, stats } => {
            let db = db::Database::open(&grove_path)?;
            display::tree(&db, &grove_path, depth, stats)
        }
        Command::Grow {
            path,
            description,
            from,
        } => {
            let db = db::Database::open(&grove_path)?;
            tree::grow(&db, &grove_path, &path, description.as_deref(), from.as_deref())
        }
        Command::Show { path } => {
            let db = db::Database::open(&grove_path)?;
            tree::show(&db, &grove_path, &path)
        }
        Command::List { flat } => {
            let db = db::Database::open(&grove_path)?;
            display::list(&db, flat)
        }
        Command::Observe {
            path,
            outcome,
            context,
            suggestion,
        } => {
            let db = db::Database::open(&grove_path)?;
            observe::record(&db, &path, &outcome, context.as_deref(), suggestion.as_deref())
        }
        Command::Merge { source, into } => {
            let db = db::Database::open(&grove_path)?;
            merge::run(&db, &grove_path, &source, into.as_deref())
        }
        Command::Health { path } => {
            let db = db::Database::open(&grove_path)?;
            observe::health(&db, path.as_deref())
        }
        Command::Stats => {
            let db = db::Database::open(&grove_path)?;
            observe::stats(&db)
        }
        Command::Prune { path, archive } => {
            let db = db::Database::open(&grove_path)?;
            tree::prune(&db, &grove_path, &path, archive)
        }
        Command::History { path } => {
            let db = db::Database::open(&grove_path)?;
            tree::history(&db, &path)
        }
        Command::Evolve { path, dry_run } => {
            let db = db::Database::open(&grove_path)?;
            tree::evolve(&db, &grove_path, &path, dry_run)
        }
        Command::Sync { output } => {
            let db = db::Database::open(&grove_path)?;
            tree::sync_plugin(&db, &grove_path, output.as_deref())
        }
    }
}
