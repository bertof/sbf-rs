use std::{io::Cursor, ops};

use byteorder::ReadBytesExt;
#[cfg(feature = "md4_hash")]
use md4::Digest;
use num::{
    Bounded,
    cast::AsPrimitive,
    FromPrimitive,
    ToPrimitive,
    Unsigned,
    Zero,
};
use rand::{Rng, rngs::OsRng};
use rayon::{iter::repeatn, prelude::*};
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
    cells: usize,
    /// Number of hash functions
    hash_number: usize,
    members: usize,
    /// Number of collisions occurred
    collisions: usize,
    safeness: f64,
    /// Number of areas
    area_number: usize,
    /// Number of members per area
    area_members: Vec<usize>,
    area_expected_cells: Vec<i64>,
    area_cells: Vec<usize>,
    area_self_collisions: Vec<usize>,
    area_prior_fpp: Vec<f64>,
    area_fpp: Vec<f64>,
    area_prior_isep: Vec<f64>,
    area_isep: Vec<f64>,
    area_prior_safep: Vec<f64>,
}

#[cfg(feature = "metrics")]
impl Metrics {
    /// Returns the number of inserted elements for the input area
    pub fn get_area_members(&self, index: usize) -> usize {
        self.area_members[index].clone()
    }
    /// Returns the sparsity of the entire SBF
    pub fn get_filter_sparsity(&self) -> f64 {
        let sum: usize = self.area_cells
            .par_iter()
            .skip(1) // Skip the index 0
            .cloned()
            .sum();
        1.0 - (sum as f64 / self.hash_number as f64)
    }
    /// Returns the posterior false positive probability over the entire filter
    /// (i.e. not area-specific)
    pub fn get_filter_fpp(&self) -> f64 {
        let non_zero_cells: usize = self.area_cells.par_iter().cloned().sum();
        let p = non_zero_cells as f64 / self.cells as f64;
        p.powi(self.hash_number as i32)
    }
    /// Returns the expected emersion value for the input area
    pub fn get_expected_area_emersion(&self, area: usize) -> f64 {
        let cells_with_greater_area_index: usize = self.area_members
            .par_iter()
            .skip(area + 1)
            .sum();
        let p = 1.0 - 1.0 / self.cells as f64;
        p.powi((self.hash_number * cells_with_greater_area_index) as i32)
    }
    pub fn get_area_emersion(&self, area: usize) -> Option<f64> {
        if self.area_cells[area] == 0 || self.hash_number == 0 {
            None
        } else {
            let a = self.area_cells[area] as f64;
            let b = (self.area_members[area] * self.hash_number -
                self.area_self_collisions[area]) as f64;
            Some(a / b)
        }
    }
    pub fn get_filter_prior_fpp(&self) -> f64 {
        let p = 1.0 - 1.0 / self.cells as f64;
        let p = 1.0 - p.powf(self.hash_number.clone() as f64 * self.members as f64);
        p.powf(self.hash_number.clone() as f64)
    }
    pub fn set_area_fpp(&mut self) {
        println!("AREA NUMBER: {}", self.area_number);
        (1..self.area_number)
            .rev()
            .for_each(|i| {
                let c: usize = (i..self.area_number)
                    .map(|j| self.area_cells[j])
                    .sum();

                let p = c as f64 / self.cells as f64;
                let p = p.powi(self.hash_number as i32);

                self.area_fpp[i] = p;

                (i..self.area_number - 1)
                    .for_each(|j| {
                        self.area_fpp[i] -= self.area_fpp[j + 1];
                    });
                self.area_fpp[i] = self.area_fpp[i].max(0.0);
            })
    }
    pub fn set_prior_area_fpp(&mut self) {
        (1..self.area_number)
            .rev()
            .for_each(|i| {
                let c: usize = (i..self.area_number)
                    .map(|j| self.area_members[j])
                    .sum();

                let p = 1.0 - 1.0 / self.cells as f64;
                let p = 1.0 - p.powi((self.hash_number * c) as i32);
                let p = p.powi(self.hash_number as i32);

                self.area_fpp[i] = p;

                (i..self.area_number - 1)
                    .for_each(|j| {
                        self.area_prior_fpp[i] -= self.area_prior_fpp[j + 1];
                    });
                self.area_prior_fpp[i] = self.area_prior_fpp[i].max(0.0);
            })
    }
    pub fn set_area_isep(&mut self) {
        (1..self.area_number)
            .rev()
            .for_each(|i| {
                let p = 1.0 - self.get_area_emersion(i).unwrap_or(-1.0);
                let p = p.powi(self.hash_number as i32);

                self.area_isep[i] = p;
            })
    }
    pub fn set_prior_area_isep(&mut self) {
        let mut p3 = 1.0;
        (1..self.area_number)
            .rev()
            .for_each(|i| {
                let n_fill: usize = (i..self.area_number)
                    .skip(1) // first element
                    .map(|j| self.area_members[j])
                    .sum();

                let p1 = 1.0 - 1.0 / self.cells as f64;
                let p1 = 1.0 - p1.powi((self.hash_number * n_fill) as i32);
                let p1 = p1.powi(self.area_members[i] as i32);

                let p2 = (1.0 - p1).powi(self.area_members[i] as i32);

                p3 *= p2;

                self.area_prior_isep[i] = p1;
                self.area_prior_safep[i] = p2;
            });

        self.safeness = p3;
    }
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
    Unsigned + Bounded + Zero + FromPrimitive + ToPrimitive + ops::AddAssign + ops::SubAssign,
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
            .map(|salt: &Salt| {
                // Iter over salt u8 values
                let salt_iterator = salt.par_iter().cloned();

                // Repeat 0, the length of the salt is the upper bound
                let zeros = repeatn(&0, salt.len());

                // XORed name
                let xor_name: Vec<u8> = name.as_bytes()
                    .par_iter()
                    .chain(zeros)
                    .zip(salt_iterator)
                    .map(|(h, v)| h ^ v)
                    .collect();

                debug_assert_eq!(xor_name.len(), salt.len());

                // First 8 u8 of the hash
                let digest = self
                    .hash(&xor_name)
                    .drain(0..8)
                    .collect::<Vec<u8>>();
                (Cursor::new(digest).read_u64::<byteorder::NativeEndian>().unwrap() as usize
                    % self.filter.len()).as_()
            })
            .collect::<Vec<U>>()
    }
    fn get_cell(&self, index: U) -> Result<&U, common::Error> {
        self.filter.get(index.to_usize().unwrap()).ok_or(IndexOutOfBounds)
    }
    fn set_cell(&mut self, index: U, area: U) -> Result<&U, common::Error> {
        if let Some(v) = self.filter.get_mut(index.to_usize().unwrap()) {
            if *v == U::zero() {
                // Cell is not marked
                *v = area;
                #[cfg(feature = "metrics")] {
                    self.metrics.area_cells[area.to_usize().unwrap()] += 1;
                }
            } else if *v < area {
                // Cell hash lower value than the input area

                #[cfg(feature = "metrics")] {
                    // v is not zero ()
                    if area > U::zero() {
                        self.metrics.area_cells[v.to_usize().unwrap()] -= 1;
                    }
                    self.metrics.area_cells[area.to_usize().unwrap()] += 1;
                    self.metrics.collisions += 1;
                }

                *v = area;
            } else if *v == area {
                // Cell hash same value than input area
                #[cfg(feature = "metrics")] {
                    self.metrics.collisions += 1;
                    self.metrics.area_self_collisions[v.to_usize().unwrap()] += 1;
                }
            } else if *v > area {
                // Cell hash higher value than input area
                #[cfg(feature = "metrics")] {
                    self.metrics.collisions += 1;
                }
            }

            Ok(v)
        } else {
            Err(IndexOutOfBounds)
        }
    }

    #[allow(unused_variables)]
    pub fn new(
        cells: U,
        hash_number: usize,
        max_input_size: usize,
        hash_function: HashFunction,
        area_number: U,
    ) -> Result<Self, common::Error> {

        // Cryptography safe RNG
        let mut rng = OsRng;

        // Generate hash salts
        let salts = (0..hash_number).map(|_| {
            (0..max_input_size).map(|_| rng.gen()).collect::<Salt>()
        }).collect::<Vec<Salt>>();

        Ok(SBF {
            filter: vec![U::zero(); cells.to_usize().ok_or(IndexOutOfBounds)?],
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
                area_members: vec![0; area_number.to_usize().ok_or(IndexOutOfBounds)?],
                area_cells: vec![0; area_number.to_usize().ok_or(IndexOutOfBounds)?],
                area_expected_cells: vec![-1; area_number.to_usize().ok_or(IndexOutOfBounds)?],
                area_self_collisions: vec![0; area_number.to_usize().ok_or(IndexOutOfBounds)?],
                area_fpp: vec![-1.0; area_number.to_usize().ok_or(IndexOutOfBounds)?],
                area_isep: vec![-1.0; area_number.to_usize().ok_or(IndexOutOfBounds)?],
                area_prior_fpp: vec![-1.0; area_number.to_usize().ok_or(IndexOutOfBounds)?],
                area_prior_isep: vec![-1.0; area_number.to_usize().ok_or(IndexOutOfBounds)?],
                area_prior_safep: vec![-1.0; area_number.to_usize().ok_or(IndexOutOfBounds)?],
            },
        })
    }

    pub fn check(&self, name: &str) -> Result<&U, common::Error> {
        self.calc_indexes(name)
            .par_iter()
            .map(|i| self.get_cell(*i))
            .try_reduce_with(|a, b| Ok(a.min(b)))
            .expect("Some value, since the iterator is not empty")
    }

    pub fn insert(&mut self, name: &str, area: U) -> Result<(), common::Error> {
        self.calc_indexes(name)
            .iter()
            .map(|i| self.set_cell(*i, area).map(|_| ()))
            .collect::<Result<(), common::Error>>()
            .and_then(|_| {
                #[cfg(feature = "metrics")] {
                    self.metrics.members += 1;
                    self.metrics.area_members[area.to_usize().unwrap()] += 1;
                }
                Ok(())
            })
    }
}

#[cfg(test)]
mod tests {
    use std::error::Error;

    use rayon::prelude::*;

    use crate::sbf::{HashFunction, SBF};

    #[test]
    fn test_sbf() -> Result<(), Box<dyn Error>> {
        let mut sbf = SBF::new(10 as u8, 2, 5,
                               HashFunction::MD5, 3)?;
        println!("{}", serde_json::to_string(&sbf)?);
        assert!(sbf.filter.par_iter().all(|v| *v == 0));

        sbf.insert("test", 1).expect("Correct insertion of an area");
        println!("{}", serde_json::to_string(&sbf)?);
        let count = sbf.filter.par_iter().cloned().filter(|v| *v == 1).count();
        assert!(2 >= count && count > 0);
        let filter = sbf.filter.clone();

        sbf.insert("test", 1).expect("Correct insertion of an area");
        println!("{}", serde_json::to_string(&sbf)?);
        assert_eq!(filter, sbf.filter);

        sbf.insert("test1", 2).expect("Correct insertion of an area");
        println!("{}", serde_json::to_string(&sbf)?);
        let filter = sbf.filter.clone();

        sbf.insert("test1", 2).expect("Correct insertion of an area");
        println!("{}", serde_json::to_string(&sbf)?);
        assert_eq!(filter, sbf.filter);

        #[cfg(feature = "metrics")] {
            sbf.metrics.set_area_fpp();
            sbf.metrics.set_prior_area_fpp();
            sbf.metrics.set_area_isep();
            sbf.metrics.set_prior_area_isep();

            println!("AREA MEMBERS: {:?}", sbf.metrics.get_area_members(1));
            assert_eq!(2, sbf.metrics.get_area_members(1));

            println!("FILTER SPARSITY: {}", sbf.metrics.get_filter_sparsity());
            println!("FILTER FPP: {}", sbf.metrics.get_filter_fpp());
            println!("EXPECTED AREA EMERSION 1: {}", sbf.metrics.get_expected_area_emersion(1));
            println!("AREA EMERSION 1: {}", sbf.metrics.get_area_emersion(1).unwrap_or(-1.0));
            println!("FILTER PRIOR FPP: {}", sbf.metrics.get_filter_prior_fpp());
        }

        Ok(())
    }
}