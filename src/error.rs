//! Convenience error module

#[cfg(feature = "serde_support")]
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Custom error definitions
#[derive(Clone, Copy, Debug, Eq, PartialEq, Error)]
#[cfg_attr(feature = "serde_support", derive(Serialize, Deserialize))]
pub enum Error {
    /// Access index is larger than the maximum size allowed
    #[error("Index out of bounds")]
    IndexOutOfBounds,
}
