use std::cmp::min;
use std::collections::HashMap;
use std::error::Error;
use std::io::Cursor;
use std::iter::repeat;

use byteorder::ReadBytesExt;
use itertools::Itertools;
#[cfg(feature = "md4_hash")]
use md4::Digest;
use num::{Bounded, FromPrimitive, ToPrimitive, Unsigned, Zero};
use num::cast::AsPrimitive;
use rand::Rng;
use rand::rngs::OsRng;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};

use crate::common::{self, Error::IndexOutOfBounds};

pub type Salt = Vec<u8>;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum HashFunction {
    #[cfg(feature = "md5_hash")]
    MD5,
    #[cfg(feature = "md4_hash")]
    MD4,
}

#[cfg(feature = "metrics")]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Metrics {
    members: usize,
    collisions: usize,
    safeness: f64,
    area_members: Vec<usize>,
    area_expected_cells: Vec<i64>,
    area_cells: Vec<usize>,
    area_self_collisions: Vec<usize>,
    area_prior_fpp: Vec<f64>,
    area_fpp: Vec<f64>,
    area_prior_isep: Vec<f64>,
    area_isep: Vec<f64>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SBF<U> where U: Unsigned + Bounded + Clone + Copy + PartialOrd + Eq {
    salts: Vec<Salt>,
    filter: Vec<U>,
    hash_function: HashFunction,
    #[cfg(feature = "metrics")]
    metrics: Metrics,
}

impl<U> SBF<U> where
    U: 'static + Send + Sync + Clone + Copy + Ord + PartialOrd + Eq +
    Unsigned + Bounded + Zero + FromPrimitive + ToPrimitive,
    usize: num::cast::AsPrimitive<U> {
    fn hash(&self, buff: &Vec<u8>) -> Vec<u8> {
        match &self.hash_function {
            #[cfg(feature = "md5_hash")]
            HashFunction::MD5 => md5::compute(buff).to_vec(),
            #[cfg(feature = "md4_hash")]
            HashFunction::MD4 => md4::Md4::digest(buff).to_vec(),
        }
    }
    fn calc_indexes(&self, name: &str) -> Vec<U> {
        self
            .salts
            .par_iter()
            .map(|salt| {
                let buffer: Vec<u8> = name
                    .as_bytes()
                    .par_iter()
                    .zip(salt.iter().chain(repeat(&0)))
                    .map(|(&h, &v)| h ^ v)
                    .collect();
                let digest = self.hash(&buffer).drain(0..8)
                    .collect::<Vec<u8>>();
                (Cursor::new(digest).read_u64::<byteorder::NativeEndian>()
                    .unwrap() as usize % self.filter.len()).as_()
            })
            .collect::<Vec<U>>()
    }
    fn get_cell(&self, index: U) -> Result<&U, common::Error> {
        self.filter.get(index.to_usize().unwrap()).ok_or(IndexOutOfBounds)
    }
    fn set_cell(&mut self, index: U, area: U) -> Result<&U, common::Error> {
        if let Some(v) = self.filter.get_mut(index.to_usize().unwrap()) {
            if *v < area {
                *v = area;
            }
            Ok(v)
        } else {
            Err(IndexOutOfBounds)
        }
    }

    pub fn new(
        cells: U,
        hashes: usize,
        max_input_size: usize,
        hash_function: HashFunction,
    ) -> Result<Self, common::Error> {

        // Cryptography safe RNG
        let mut rng = OsRng;

        // Generate hash salts
        let salts = (0..hashes).map(|_| {
            (0..max_input_size).map(|_| rng.gen()).collect::<Salt>()
        }).collect::<Vec<Salt>>();

        let n_cells = cells.to_usize().ok_or(IndexOutOfBounds)?;

        Ok(SBF {
            filter: vec![U::zero(); n_cells],
            hash_function,
            salts,

            #[cfg(feature = "metrics")]
            metrics: Metrics {
                members: 0,
                collisions: 0,
                safeness: 0.0,
                area_members: vec![0; n_cells],
                area_cells: vec![0; n_cells],
                area_expected_cells: vec![-1; n_cells],
                area_self_collisions: vec![0; n_cells],
                area_fpp: vec![-1.0; n_cells],
                area_isep: vec![-1.0; n_cells],
                area_prior_fpp: vec![-1.0; n_cells],
                area_prior_isep: vec![-1.0; n_cells],
            },
        })
    }

    pub fn check(&self, name: &str) -> Result<&U, common::Error> {
        self.calc_indexes(name)
            .par_iter()
            .map(|i| self.get_cell(*i))
            .try_reduce_with(|a, b| Ok(min(a, b)))
            .expect("Some value, since the iterator is not empty")
    }

    pub fn insert(&mut self, name: &str, area: U) -> Result<(), common::Error> {
        self.calc_indexes(name)
            .iter()
            .map(|i| self.set_cell(*i, area).map(|_| ()))
            .collect()
    }


    #[cfg(feature = "metrics")]
    pub fn get_area_members(index: U) -> usize {
        unimplemented!()
    }
    #[cfg(feature = "metrics")]
    pub fn get_filter_sparsity(&self) -> f64 { unimplemented!() }
    #[cfg(feature = "metrics")]
    pub fn get_filter_fpp(&self) -> f64 { unimplemented!() }
    #[cfg(feature = "metrics")]
    pub fn get_filter_a_priori_fpp(&self) -> f64 { unimplemented!() }
    #[cfg(feature = "metrics")]
    pub fn set_area_fpp(&mut self) { unimplemented!() }
    #[cfg(feature = "metrics")]
    pub fn set_prior_area_fpp(&mut self) { unimplemented!() }
    #[cfg(feature = "metrics")]
    pub fn set_area_isep(&mut self) { unimplemented!() }
    #[cfg(feature = "metrics")]
    pub fn set_prior_area_isep(&mut self) { unimplemented!() }
    #[cfg(feature = "metrics")]
    pub fn get_expected_area_emersion(&self, index: U) -> f64 { unimplemented!() }
    #[cfg(feature = "metrics")]
    pub fn get_area_emersion(&self, index: U) -> f64 { unimplemented!() }
}

#[cfg(test)]
mod tests {
    use std::error::Error;

    use crate::common::Error::IndexOutOfBounds;
    use crate::sbf::{HashFunction, SBF};

    #[test]
    fn test_sbf() -> Result<(), Box<dyn Error>> {
        let mut sbf = SBF::new(10 as u8, 2, 5, HashFunction::MD5)?;
        println!("{}", serde_json::to_string(&sbf)?);

        sbf.insert("test", 1).expect("Correct insertion of an area");
        println!("{}", serde_json::to_string(&sbf)?);

        sbf.insert("test", 1).expect("Correct insertion of an area");
        println!("{}", serde_json::to_string(&sbf)?);

        sbf.insert("test1", 2).expect("Correct insertion of an area");
        println!("{}", serde_json::to_string(&sbf)?);

        sbf.insert("test12", 2).expect("Correct insertion of an area");
        println!("{}", serde_json::to_string(&sbf)?);


        Ok(())
    }
}