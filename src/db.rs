use anyhow::{Context, Result};
use chrono::Utc;
use rusqlite::{params, Connection};
use std::path::Path;
use uuid::Uuid;

use crate::error::GroveError;

pub struct Database {
    conn: Connection,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct Skill {
    pub id: String,
    pub name: String,
    pub path: String,
    pub description: String,
    pub version: u32,
    pub parent_id: Option<String>,
    pub source: String,
    pub usage_count: u64,
    pub success_count: u64,
    pub failure_count: u64,
    pub created_at: String,
    pub evolved_at: Option<String>,
    pub content_hash: String,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct Observation {
    pub id: String,
    pub skill_id: String,
    pub outcome: String,
    pub context: Option<String>,
    pub suggestion: Option<String>,
    pub created_at: String,
    pub consumed: bool,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct VersionRecord {
    pub id: String,
    pub skill_id: String,
    pub version: u32,
    pub content_hash: String,
    pub description: String,
    pub created_at: String,
}

impl Database {
    pub fn open(grove_path: &Path) -> Result<Self> {
        let db_path = grove_path.join("grove.db");
        if !db_path.exists() {
            anyhow::bail!(GroveError::NotInitialized(
                grove_path.display().to_string()
            ));
        }
        let conn = Connection::open(&db_path)
            .with_context(|| format!("failed to open grove.db at {}", db_path.display()))?;
        Ok(Database { conn })
    }

    pub fn create(grove_path: &Path) -> Result<Self> {
        let db_path = grove_path.join("grove.db");
        let conn = Connection::open(&db_path)
            .with_context(|| format!("failed to create grove.db at {}", db_path.display()))?;

        conn.execute_batch(SCHEMA)?;
        Ok(Database { conn })
    }

    // --- Skill CRUD ---

    pub fn insert_skill(
        &self,
        name: &str,
        path: &str,
        description: &str,
        parent_id: Option<&str>,
        source: &str,
        content_hash: &str,
    ) -> Result<String> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        self.conn.execute(
            "INSERT INTO skills (id, name, path, description, version, parent_id, source, usage_count, success_count, failure_count, created_at, content_hash)
             VALUES (?1, ?2, ?3, ?4, 1, ?5, ?6, 0, 0, 0, ?7, ?8)",
            params![id, name, path, description, parent_id, source, now, content_hash],
        )?;

        // Insert initial version record
        let ver_id = Uuid::new_v4().to_string();
        self.conn.execute(
            "INSERT INTO versions (id, skill_id, version, content_hash, description, created_at)
             VALUES (?1, ?2, 1, ?3, 'initial', ?4)",
            params![ver_id, id, content_hash, now],
        )?;

        Ok(id)
    }

    pub fn get_skill_by_path(&self, path: &str) -> Result<Option<Skill>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, path, description, version, parent_id, source, usage_count, success_count, failure_count, created_at, evolved_at, content_hash
             FROM skills WHERE path = ?1 AND archived = 0"
        )?;

        let result = stmt.query_row(params![path], |row| {
            Ok(Skill {
                id: row.get(0)?,
                name: row.get(1)?,
                path: row.get(2)?,
                description: row.get(3)?,
                version: row.get(4)?,
                parent_id: row.get(5)?,
                source: row.get(6)?,
                usage_count: row.get(7)?,
                success_count: row.get(8)?,
                failure_count: row.get(9)?,
                created_at: row.get(10)?,
                evolved_at: row.get(11)?,
                content_hash: row.get(12)?,
            })
        });

        match result {
            Ok(skill) => Ok(Some(skill)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    pub fn list_skills(&self) -> Result<Vec<Skill>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, path, description, version, parent_id, source, usage_count, success_count, failure_count, created_at, evolved_at, content_hash
             FROM skills WHERE archived = 0 ORDER BY path"
        )?;

        let skills = stmt.query_map([], |row| {
            Ok(Skill {
                id: row.get(0)?,
                name: row.get(1)?,
                path: row.get(2)?,
                description: row.get(3)?,
                version: row.get(4)?,
                parent_id: row.get(5)?,
                source: row.get(6)?,
                usage_count: row.get(7)?,
                success_count: row.get(8)?,
                failure_count: row.get(9)?,
                created_at: row.get(10)?,
                evolved_at: row.get(11)?,
                content_hash: row.get(12)?,
            })
        })?.collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(skills)
    }

    pub fn archive_skill(&self, path: &str) -> Result<bool> {
        let rows = self.conn.execute(
            "UPDATE skills SET archived = 1 WHERE path = ?1 AND archived = 0",
            params![path],
        )?;
        Ok(rows > 0)
    }

    pub fn delete_skill(&self, path: &str) -> Result<bool> {
        let rows = self.conn.execute(
            "DELETE FROM skills WHERE path = ?1",
            params![path],
        )?;
        Ok(rows > 0)
    }

    pub fn update_skill_content(&self, path: &str, content_hash: &str, description: &str) -> Result<bool> {
        let now = Utc::now().to_rfc3339();
        // Get current version
        let skill = self.get_skill_by_path(path)?;
        let skill = match skill {
            Some(s) => s,
            None => return Ok(false),
        };
        let new_version = skill.version + 1;

        self.conn.execute(
            "UPDATE skills SET version = ?1, content_hash = ?2, evolved_at = ?3 WHERE path = ?4 AND archived = 0",
            params![new_version, content_hash, now, path],
        )?;

        // Insert version record
        let ver_id = Uuid::new_v4().to_string();
        self.conn.execute(
            "INSERT INTO versions (id, skill_id, version, content_hash, description, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![ver_id, skill.id, new_version, content_hash, description, now],
        )?;

        Ok(true)
    }

    // --- Observations ---

    pub fn insert_observation(
        &self,
        skill_id: &str,
        outcome: &str,
        context: Option<&str>,
        suggestion: Option<&str>,
    ) -> Result<String> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        self.conn.execute(
            "INSERT INTO observations (id, skill_id, outcome, context, suggestion, created_at, consumed)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, 0)",
            params![id, skill_id, outcome, context, suggestion, now],
        )?;

        // Update skill counters
        match outcome {
            "success" => {
                self.conn.execute(
                    "UPDATE skills SET usage_count = usage_count + 1, success_count = success_count + 1 WHERE id = ?1",
                    params![skill_id],
                )?;
            }
            "failure" => {
                self.conn.execute(
                    "UPDATE skills SET usage_count = usage_count + 1, failure_count = failure_count + 1 WHERE id = ?1",
                    params![skill_id],
                )?;
            }
            _ => {
                self.conn.execute(
                    "UPDATE skills SET usage_count = usage_count + 1 WHERE id = ?1",
                    params![skill_id],
                )?;
            }
        }

        Ok(id)
    }

    pub fn get_observations(&self, skill_id: &str, unconsumed_only: bool) -> Result<Vec<Observation>> {
        let sql = if unconsumed_only {
            "SELECT id, skill_id, outcome, context, suggestion, created_at, consumed
             FROM observations WHERE skill_id = ?1 AND consumed = 0 ORDER BY created_at"
        } else {
            "SELECT id, skill_id, outcome, context, suggestion, created_at, consumed
             FROM observations WHERE skill_id = ?1 ORDER BY created_at"
        };

        let mut stmt = self.conn.prepare(sql)?;
        let obs = stmt.query_map(params![skill_id], |row| {
            Ok(Observation {
                id: row.get(0)?,
                skill_id: row.get(1)?,
                outcome: row.get(2)?,
                context: row.get(3)?,
                suggestion: row.get(4)?,
                created_at: row.get(5)?,
                consumed: row.get(6)?,
            })
        })?.collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(obs)
    }

    pub fn mark_observations_consumed(&self, skill_id: &str) -> Result<usize> {
        let rows = self.conn.execute(
            "UPDATE observations SET consumed = 1 WHERE skill_id = ?1 AND consumed = 0",
            params![skill_id],
        )?;
        Ok(rows)
    }

    // --- Versions ---

    pub fn get_versions(&self, skill_id: &str) -> Result<Vec<VersionRecord>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, skill_id, version, content_hash, description, created_at
             FROM versions WHERE skill_id = ?1 ORDER BY version DESC"
        )?;

        let versions = stmt.query_map(params![skill_id], |row| {
            Ok(VersionRecord {
                id: row.get(0)?,
                skill_id: row.get(1)?,
                version: row.get(2)?,
                content_hash: row.get(3)?,
                description: row.get(4)?,
                created_at: row.get(5)?,
            })
        })?.collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(versions)
    }

    // --- Stats ---

    pub fn skill_count(&self) -> Result<u64> {
        let count: u64 = self.conn.query_row(
            "SELECT COUNT(*) FROM skills WHERE archived = 0",
            [],
            |row| row.get(0),
        )?;
        Ok(count)
    }

    pub fn observation_count(&self) -> Result<u64> {
        let count: u64 = self.conn.query_row(
            "SELECT COUNT(*) FROM observations",
            [],
            |row| row.get(0),
        )?;
        Ok(count)
    }

    pub fn total_usage(&self) -> Result<u64> {
        let count: u64 = self.conn.query_row(
            "SELECT COALESCE(SUM(usage_count), 0) FROM skills WHERE archived = 0",
            [],
            |row| row.get(0),
        )?;
        Ok(count)
    }

    pub fn total_successes(&self) -> Result<u64> {
        let count: u64 = self.conn.query_row(
            "SELECT COALESCE(SUM(success_count), 0) FROM skills WHERE archived = 0",
            [],
            |row| row.get(0),
        )?;
        Ok(count)
    }

    pub fn total_failures(&self) -> Result<u64> {
        let count: u64 = self.conn.query_row(
            "SELECT COALESCE(SUM(failure_count), 0) FROM skills WHERE archived = 0",
            [],
            |row| row.get(0),
        )?;
        Ok(count)
    }

    pub fn top_skills(&self, limit: usize) -> Result<Vec<Skill>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, path, description, version, parent_id, source, usage_count, success_count, failure_count, created_at, evolved_at, content_hash
             FROM skills WHERE archived = 0 ORDER BY usage_count DESC LIMIT ?1"
        )?;

        let skills = stmt.query_map(params![limit as u32], |row| {
            Ok(Skill {
                id: row.get(0)?,
                name: row.get(1)?,
                path: row.get(2)?,
                description: row.get(3)?,
                version: row.get(4)?,
                parent_id: row.get(5)?,
                source: row.get(6)?,
                usage_count: row.get(7)?,
                success_count: row.get(8)?,
                failure_count: row.get(9)?,
                created_at: row.get(10)?,
                evolved_at: row.get(11)?,
                content_hash: row.get(12)?,
            })
        })?.collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(skills)
    }

    pub fn struggling_skills(&self, limit: usize) -> Result<Vec<Skill>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, path, description, version, parent_id, source, usage_count, success_count, failure_count, created_at, evolved_at, content_hash
             FROM skills WHERE archived = 0 AND usage_count > 0 ORDER BY (CAST(failure_count AS REAL) / usage_count) DESC LIMIT ?1"
        )?;

        let skills = stmt.query_map(params![limit as u32], |row| {
            Ok(Skill {
                id: row.get(0)?,
                name: row.get(1)?,
                path: row.get(2)?,
                description: row.get(3)?,
                version: row.get(4)?,
                parent_id: row.get(5)?,
                source: row.get(6)?,
                usage_count: row.get(7)?,
                success_count: row.get(8)?,
                failure_count: row.get(9)?,
                created_at: row.get(10)?,
                evolved_at: row.get(11)?,
                content_hash: row.get(12)?,
            })
        })?.collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(skills)
    }
}

const SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS skills (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    path TEXT NOT NULL UNIQUE,
    description TEXT NOT NULL DEFAULT '',
    version INTEGER NOT NULL DEFAULT 1,
    parent_id TEXT,
    source TEXT NOT NULL DEFAULT 'native',
    usage_count INTEGER NOT NULL DEFAULT 0,
    success_count INTEGER NOT NULL DEFAULT 0,
    failure_count INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    evolved_at TEXT,
    content_hash TEXT NOT NULL DEFAULT '',
    archived INTEGER NOT NULL DEFAULT 0,
    FOREIGN KEY (parent_id) REFERENCES skills(id)
);

CREATE INDEX IF NOT EXISTS idx_skills_path ON skills(path);
CREATE INDEX IF NOT EXISTS idx_skills_parent ON skills(parent_id);
CREATE INDEX IF NOT EXISTS idx_skills_archived ON skills(archived);

CREATE TABLE IF NOT EXISTS observations (
    id TEXT PRIMARY KEY,
    skill_id TEXT NOT NULL,
    outcome TEXT NOT NULL,
    context TEXT,
    suggestion TEXT,
    created_at TEXT NOT NULL,
    consumed INTEGER NOT NULL DEFAULT 0,
    FOREIGN KEY (skill_id) REFERENCES skills(id)
);

CREATE INDEX IF NOT EXISTS idx_observations_skill ON observations(skill_id);
CREATE INDEX IF NOT EXISTS idx_observations_consumed ON observations(consumed);

CREATE TABLE IF NOT EXISTS versions (
    id TEXT PRIMARY KEY,
    skill_id TEXT NOT NULL,
    version INTEGER NOT NULL,
    content_hash TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    created_at TEXT NOT NULL,
    FOREIGN KEY (skill_id) REFERENCES skills(id)
);

CREATE INDEX IF NOT EXISTS idx_versions_skill ON versions(skill_id);

CREATE TABLE IF NOT EXISTS grove_meta (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

INSERT OR IGNORE INTO grove_meta (key, value) VALUES ('version', '0.1.0');
INSERT OR IGNORE INTO grove_meta (key, value) VALUES ('created_at', datetime('now'));
"#;
