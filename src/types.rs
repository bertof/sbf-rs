//! Common types for the SBF data structure

#[cfg(feature = "serde_support")]
use serde::{Deserialize, Serialize};

/// Salt string type
///
/// We use a `u8` encoding for the hash string.
pub type Salt = Vec<u8>;

/// The kind of hashing function that is used by the data structure
///
/// By default only MD5 is enabled, MD4 can be enabled by using the `md4_hash` feature.
#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "serde_support", derive(Serialize, Deserialize))]
pub enum HashFunction {
    /// MD5 hash function
    #[cfg(feature = "md5_hash")]
    MD5,
    /// MD4 hash function
    #[cfg(feature = "md4_hash")]
    MD4,
}