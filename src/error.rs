//! Convenience error module

#[cfg(feature = "serde_support")]
use serde::{Deserialize, Serialize};

/// Custom error definitions
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "serde_support", derive(Serialize, Deserialize))]
pub enum Error {
    /// Access index is larger than the maximum size allowed
    IndexOutOfBounds,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", match &self {
            Error::IndexOutOfBounds => "Index out of bounds",
        })
    }
}

impl std::error::Error for Error {}