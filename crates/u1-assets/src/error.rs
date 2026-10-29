use std::fmt;

/// Errors produced while locating or parsing game assets.
#[derive(Debug)]
pub enum AssetError {
    Io(std::io::Error),
    /// A required file was not found in the game directory.
    NotFound(String),
    /// A file had a size inconsistent with the documented format.
    BadSize { what: String, expected: String, got: usize },
    /// Invalid parameters supplied to a decoder.
    Invalid(String),
}

pub type Result<T> = std::result::Result<T, AssetError>;

impl fmt::Display for AssetError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AssetError::Io(e) => write!(f, "I/O error: {e}"),
            AssetError::NotFound(n) => write!(f, "file not found in game directory: {n}"),
            AssetError::BadSize { what, expected, got } => {
                write!(f, "{what}: unexpected size {got} bytes (expected {expected})")
            }
            AssetError::Invalid(m) => write!(f, "invalid: {m}"),
        }
    }
}

impl std::error::Error for AssetError {}

impl From<std::io::Error> for AssetError {
    fn from(e: std::io::Error) -> Self {
        AssetError::Io(e)
    }
}
