//! SBF is a probabilistic data structure that maps elements of a space to indexed disjoint subsets
//! of that space.
//!
//! This is a reimplementation of the [C library](https://github.com/spatialbloomfilter/libSBF-cpp)
//! by the original research group.

#![warn(
// Harden built-in lints
missing_copy_implementations,
missing_debug_implementations,
missing_docs,
unreachable_pub,

// Harden clippy lints
clippy::cargo_common_metadata,
clippy::clone_on_ref_ptr,
clippy::dbg_macro,
clippy::decimal_literal_representation,
clippy::float_cmp_const,
clippy::get_unwrap,
clippy::integer_arithmetic,
clippy::integer_division,
clippy::print_stdout,
)]

pub use {
    data_structure::SBF,
    error::Error,
    types::{HashFunction, Salt},
};
#[cfg(feature = "metrics")]
pub use metrics::Metrics;

pub mod data_structure;
pub mod error;
#[cfg(feature = "metrics")]
pub mod metrics;
pub mod types;

#[cfg(test)]
mod tests;

