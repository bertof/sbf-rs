//! SBF data structure module

use std::sync::Mutex;
use std::{io::Cursor, ops};

use byteorder::ReadBytesExt;
#[cfg(feature = "md4_hash")]
use md4::Digest;
#[cfg(feature = "md5_hash")]
use md5::compute as md5_compute;
use num::{cast::AsPrimitive, Bounded, FromPrimitive, ToPrimitive, Unsigned, Zero};
use rand::{rngs::OsRng, Rng};
use rayon::{iter::repeatn, prelude::*};
#[cfg(feature = "serde_support")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "metrics")]
use crate::metrics::Metrics;
use crate::{
    error::Error,
    types::{HashFunction, Salt},
};

/// Spatial Bloom Filter data structure
///
/// This data structure uses a multi level bloom filter to identify if a content has already been
/// inserted in the filter and of which of a finite number of disjoint subsets of the origin space
/// it belongs to.
/// This is a probabilistic data structure
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde_support", derive(Serialize, Deserialize))]
pub struct SBF<U>
where
    U: Unsigned + Bounded + Clone + Copy + PartialOrd + Eq,
{
    /// Hash salt container
    salts: Vec<Salt>,
    /// Filter
    pub(crate) filter: Vec<U>,
    /// Hash function to use during the calculation of the insertion and query indexes
    hash_function: HashFunction,
    #[cfg(feature = "metrics")]
    /// SBF metrics structure
    ///
    /// Can be activated enabling the `metrics` feature.
    /// Can be queried to retrieve information about the state of the filter.
    pub metrics: Metrics,
}

impl<U> SBF<U>
where
    U: 'static
        + Send
        + Sync
        + Clone
        + Copy
        + Ord
        + PartialOrd
        + Eq
        + Unsigned
        + Bounded
        + Zero
        + FromPrimitive
        + ToPrimitive
        + ops::AddAssign
        + ops::SubAssign,
    usize: num::cast::AsPrimitive<U>,
{
    /// Adapter for the hash function used by the filter
    fn hash(&self, buff: &[u8]) -> Vec<u8> {
        match &self.hash_function {
            #[cfg(feature = "md5_hash")]
            HashFunction::MD5 => md5_compute(buff).to_vec(),
            #[cfg(feature = "md4_hash")]
            HashFunction::MD4 => md4::Md4::digest(buff).to_vec(),
        }
    }

    /// Calculates the indexed of the cells pointed by each of the hashes generated from the input
    fn calc_indexes(&self, content: Vec<u8>) -> Vec<U> {
        self.salts
            .par_iter()
            .map(|salt: &Salt| {
                // Iter over salt u8 values
                let salt_iterator = salt.par_iter();

                // Repeat 0, the length of the salt is the upper bound
                let zeros = repeatn(&(0_u8), salt.len());

                // Content input with padding
                let content = content.par_iter().chain(zeros);

                // XORed content
                let xor_content: Vec<u8> = content.zip(salt_iterator).map(|(h, v)| h ^ v).collect();

                // First 8 u8 of the hash
                let digest = self.hash(&xor_content).drain(0..8).collect::<Vec<u8>>();

                // Read digest as a u64
                let digest_value = Cursor::new(digest)
                    .read_u64::<byteorder::NativeEndian>()
                    .unwrap();

                // Return cell index
                (digest_value as usize % self.filter.len()).as_()
            })
            .collect::<Vec<U>>()
    }

    /// Returns the content of a cell
    fn get_cell(&self, index: U) -> Result<&U, Error> {
        self.filter
            .get(index.to_usize().unwrap())
            .ok_or(Error::IndexOutOfBounds)
    }

    /// Sets the content of the cell if the input area is higher than the one in the filter
    fn set_cell(&mut self, index: U, area: U) -> Result<&U, Error> {
        if let Some(v) = self.filter.get_mut(index.to_usize().unwrap()) {
            if *v == U::zero() || *v < area {
                // Cell is not marked or cell has lower value than the input area
                *v = area;
            } else if *v >= area {
                // Cell hash same or higher value than input area
            }

            #[cfg(feature = "metrics")]
            {
                if *v == U::zero() {
                    // Cell is not marked
                    self.metrics.area_cells[area.to_usize().unwrap()] += 1;
                } else if *v < area {
                    // Cell hash lower value than the input area
                    // v is not zero ()
                    if area > U::zero() {
                        self.metrics.area_cells[v.to_usize().unwrap()] -= 1;
                    }
                    self.metrics.area_cells[area.to_usize().unwrap()] += 1;
                    self.metrics.collisions += 1;
                } else if *v == area {
                    // Cell hash same value than input area
                    self.metrics.collisions += 1;
                    self.metrics.area_self_collisions[v.to_usize().unwrap()] += 1;
                } else if *v > area {
                    // Cell hash higher value than input area
                    self.metrics.collisions += 1;
                }
            }

            Ok(v)
        } else {
            Err(Error::IndexOutOfBounds)
        }
    }

    /// Constructor of the SBF data structure
    ///
    /// - `cells`: Number of cells in the filter,
    /// - `hash_number`: Number of hash functions used,
    /// - `max_input_size`: Maximum input dimension, if a larger one is used it will be truncated,
    /// - `hash_function`: Kind of hash function to use,
    /// - `area_number`: Number of different areas (only used in metrics).
    pub fn new(
        cells: U,
        hash_number: usize,
        max_input_size: usize,
        hash_function: HashFunction,
        #[cfg(feature = "metrics")] area_number: U,
    ) -> Result<Self, Error> {
        assert!(cells > U::zero());

        // Cryptography safe RNG
        let rng = Mutex::new(OsRng);

        // Generate hash salts
        let salts = (0..hash_number)
            .into_par_iter()
            .map(|_| {
                (0..max_input_size)
                    .into_par_iter()
                    .map(|_| rng.lock().unwrap().gen())
                    .collect::<Salt>()
            })
            .collect::<Vec<Salt>>();

        Ok(SBF {
            filter: vec![U::zero(); cells.to_usize().ok_or(Error::IndexOutOfBounds)?],
            hash_function,
            salts,

            #[cfg(feature = "metrics")]
            metrics: Metrics {
                cells: cells.to_usize().unwrap(),
                hash_number,
                members: 0,
                collisions: 0,
                safeness: 0.0,
                area_number: area_number.to_usize().unwrap(),
                area_members: vec![0; area_number.to_usize().ok_or(Error::IndexOutOfBounds)?],
                area_cells: vec![0; area_number.to_usize().ok_or(Error::IndexOutOfBounds)?],
                area_expected_cells: vec![
                    -1;
                    area_number.to_usize().ok_or(Error::IndexOutOfBounds)?
                ],
                area_self_collisions: vec![
                    0;
                    area_number.to_usize().ok_or(Error::IndexOutOfBounds)?
                ],
                area_fpp: vec![-1.0; area_number.to_usize().ok_or(Error::IndexOutOfBounds)?],
                area_isep: vec![-1.0; area_number.to_usize().ok_or(Error::IndexOutOfBounds)?],
                area_prior_fpp: vec![-1.0; area_number.to_usize().ok_or(Error::IndexOutOfBounds)?],
                area_prior_isep: vec![-1.0; area_number.to_usize().ok_or(Error::IndexOutOfBounds)?],
                area_prior_safep: vec![
                    -1.0;
                    area_number.to_usize().ok_or(Error::IndexOutOfBounds)?
                ],
            },
        })
    }

    /// Constructor of the SBF data structure using optimal parameters
    pub fn new_optimal(
        expected_inserts: usize,
        max_fpp: f64,
        max_input_size: usize,
        hash_function: HashFunction,
        #[cfg(feature = "metrics")] area_number: U,
    ) -> Result<Self, Error> {
        let optimal_cells =
            (-(expected_inserts as f64) * max_fpp.ln() / (2.0f64.ln().powi(2))) as u64;
        let hash_number =
            (optimal_cells as f64 / (expected_inserts as f64) * 2.0f64.ln()).ceil() as usize;
        Self::new(
            U::from_u64(optimal_cells).ok_or(Error::IndexOutOfBounds)?,
            hash_number,
            max_input_size,
            hash_function,
            #[cfg(feature = "metrics")]
            area_number,
        )
    }
    /// Check an input for presence in the filter.
    /// It will return `0` if the input is not been inserted or the index of the area it belongs to
    /// if it has been inserted previously.
    ///
    /// Because of the probabilistic nature of this data structure, it is possible for it to return
    /// a false positive.
    pub fn check(&self, content: Vec<u8>) -> Result<&U, Error> {
        self.calc_indexes(content)
            .par_iter()
            .map(|i| self.get_cell(*i))
            .try_reduce_with(|a, b| Ok(a.min(b)))
            .expect("Some value, since the iterator is not empty")
    }

    /// Insert the content in the filter associated to the given area.
    pub fn insert(&mut self, content: Vec<u8>, area: U) -> Result<(), Error> {
        self.calc_indexes(content)
            .iter()
            .try_for_each(|i| self.set_cell(*i, area).map(|_| ()))
            .map(|_| {
                #[cfg(feature = "metrics")]
                #[allow(clippy::integer_arithmetic)]
                {
                    self.metrics.members += 1;
                    self.metrics.area_members[area.to_usize().unwrap()] += 1;
                };
            })
    }
}
