use serde::{Serialize, Serializer};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Input the user can fix; the message says how.
    #[error("{0}")]
    Invalid(String),
    #[error("Not found: {0}")]
    NotFound(String),
    /// Something on this computer is missing or refused (Claude not installed, no terminal).
    #[error("{0}")]
    Unavailable(String),
    #[error("{0}")]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, Error>;

// Tauri sends errors to the UI as plain strings.
impl Serialize for Error {
    fn serialize<S: Serializer>(&self, s: S) -> std::result::Result<S::Ok, S::Error> {
        s.serialize_str(&self.to_string())
    }
}
