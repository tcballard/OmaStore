//! Private workflow authority. Published catalogue content remains Git-owned.
pub mod auth;
pub mod checks;
pub mod drafts;
pub mod github;
pub mod media;
pub mod net;
pub mod publication;
pub mod publisher;
pub mod review;
pub mod store;

use serde::Serialize;
use sha2::{Digest, Sha256};
use std::io::Read;

pub use store::{Actor, Store};
pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Error {
    pub code: &'static str,
    pub status: u16,
}
impl Error {
    pub const fn new(status: u16, code: &'static str) -> Self {
        Self { status, code }
    }
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.code)
    }
}
impl std::error::Error for Error {}
impl From<rusqlite::Error> for Error {
    fn from(_: rusqlite::Error) -> Self {
        Self::new(500, "storage_unavailable")
    }
}
impl From<std::io::Error> for Error {
    fn from(_: std::io::Error) -> Self {
        Self::new(500, "storage_unavailable")
    }
}
impl From<serde_json::Error> for Error {
    fn from(_: serde_json::Error) -> Self {
        Self::new(422, "invalid_fields")
    }
}
pub fn digest(bytes: impl AsRef<[u8]>) -> String {
    format!("{:x}", Sha256::digest(bytes.as_ref()))
}
pub fn nonce() -> Result<String> {
    let mut bytes = [0; 32];
    std::fs::File::open("/dev/urandom")?.read_exact(&mut bytes)?;
    Ok(bytes.iter().map(|b| format!("{b:02x}")).collect())
}
pub fn now() -> i64 {
    chrono::Utc::now().timestamp()
}
pub fn bounded(value: &str, max: usize) -> Result<()> {
    if value.is_empty()
        || value.len() > max
        || value
            .chars()
            .any(|c| c.is_control() && c != '\n' && c != '\t')
    {
        Err(Error::new(422, "invalid_fields"))
    } else {
        Ok(())
    }
}
