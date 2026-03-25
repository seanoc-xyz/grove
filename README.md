# Grove

A self-evolving skill tree for AI agents.

Grove is a Rust CLI that IS the skill tree — not a toolkit for writing skills, but the living tree that grows, evolves, merges, prunes, and versions skills autonomously as agents work.

## How It Works

```
Observe (usage tracking via `grove observe`)
  → Extract (pattern detection from observations)
    → Evolve (improved skill with gotchas + suggestions)
      → Version (git commit + version bump)
        → Monitor (`grove health` tracks improvement)
```

## Install

```bash
cargo install --path .
```

Or build directly:

```bash
cargo build --release
# Binary at target/release/grove
```

## Quick Start

```bash
# Plant a new grove
grove init

# Grow skills into the tree
grove grow coding/review -d "Use for code review tasks"
grove grow coding/review/security -d "Security-focused review"
grove grow debugging/systematic -d "Step-by-step debugging methodology"

# View your skill tree
grove tree
# grove
# ├── coding
# │   └── review
# │       └── security
# └── debugging
#     └── systematic

# Record usage observations
grove observe coding/review success -c "Caught race condition" -s "Add concurrency checklist"
grove observe coding/review failure -c "Missed SQL injection" -s "Add security patterns to review"

# Evolve skills from observations
grove evolve coding/review --dry-run    # preview
grove evolve coding/review              # apply

# Import skills from Claude Code plugins
grove merge ~/.claude/plugins/cache/superpowers/

# Generate Claude Code plugin from your grove
grove sync --output ~/my-plugin

# Check grove health
grove health
grove stats
```

## Commands

| Command | Description |
|---------|-------------|
| `grove init [path]` | Plant a new grove (default: `~/.grove`) |
| `grove tree [--depth N] [--stats]` | Visualize the skill tree |
| `grove grow <path> [-d desc] [--from template]` | Grow a new skill |
| `grove show <path>` | Display skill content + metadata |
| `grove list [--flat]` | List all skills with stats |
| `grove observe <path> <outcome> [-c ctx] [-s suggestion]` | Record usage observation |
| `grove evolve <path> [--dry-run]` | Evolve skill from observations |
| `grove merge <source> [--into path]` | Import external skills |
| `grove sync [--output dir]` | Generate Claude Code plugin manifest |
| `grove history <path>` | Version history of a skill |
| `grove health [path]` | Usage and health dashboard |
| `grove stats` | Overall grove statistics |
| `grove prune <path> [--archive]` | Remove or archive a skill |

## Architecture

Grove uses a triple-layer storage model:

- **Filesystem** (`tree/`) — Skills as `SKILL.md` files in a hierarchical directory structure
- **SQLite** (`grove.db`) — Metadata, usage counters, observations, version history
- **Git** — Content versioning for every skill change

### Directory Structure

```
~/.grove/
├── grove.db      # Metadata + tracking
├── .git/         # Version control
├── tree/         # Active skills
│   ├── coding/
│   │   └── review/
│   │       └── SKILL.md
│   └── debugging/
│       └── systematic/
│           └── SKILL.md
├── drafts/       # Skills being evolved
└── archive/      # Pruned skills
```

### SQLite Schema

- `skills` — Skill metadata (path, version, usage/success/failure counts, content hash)
- `observations` — Usage feedback (outcome, context, suggestions, consumed flag)
- `versions` — Version records linking skills to content hashes
- `grove_meta` — Grove-level metadata

## Claude Code Integration

1. `grove sync` generates a `.claude-plugin/plugin.json` + flattened skill files
2. Use with Claude Code: `claude --plugin-dir <output-dir>`
3. After using skills, call `grove observe` to record outcomes
4. Run `grove evolve` to improve skills based on observations
5. Git tracks every change for rollback

## Configuration

- Default grove location: `~/.grove/`
- Override with `--grove <path>` flag or `GROVE_PATH` env var

## License

MIT
