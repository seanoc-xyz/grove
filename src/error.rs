use std::fmt;

#[derive(Debug)]
#[allow(dead_code)]
pub enum GroveError {
    NotInitialized(String),
    SkillExists(String),
    SkillNotFound(String),
    InvalidPath(String),
    InvalidOutcome(String),
    DatabaseError(String),
}

impl fmt::Display for GroveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GroveError::NotInitialized(p) => write!(f, "grove not initialized at {p}. Run `grove init` first."),
            GroveError::SkillExists(p) => write!(f, "skill already exists at path: {p}"),
            GroveError::SkillNotFound(p) => write!(f, "no skill found at path: {p}"),
            GroveError::InvalidPath(p) => write!(f, "invalid skill path: {p}. Use slash-separated segments (e.g., coding/review)"),
            GroveError::InvalidOutcome(o) => write!(f, "invalid outcome: {o}. Use: success, failure, or partial"),
            GroveError::DatabaseError(e) => write!(f, "database error: {e}"),
        }
    }
}

impl std::error::Error for GroveError {}
