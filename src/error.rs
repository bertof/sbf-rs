//! Convenience error module

use std::{error, fmt};

/// Custom error definitions
#[derive(Debug, Eq, PartialEq)]
pub enum Error {
    IndexOutOfBounds,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", match &self {
            Error::IndexOutOfBounds => "Index out of bounds",
        })
    }
}

impl error::Error for Error {}