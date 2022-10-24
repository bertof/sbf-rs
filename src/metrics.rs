//! Metrics data structure module

use num::traits::Pow;
use rayon::prelude::*;
#[cfg(feature = "serde_support")]
use serde::{Deserialize, Serialize};

/// The data structure that contains the metrics about the current `SBF` structure.
///
/// This data structure is automatically added to each `SBF` if the feature `metrics` is enabled.
/// It's not necessary and is disabled by default.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde_support", derive(Serialize, Deserialize))]
pub struct Metrics {
    /// Number of cells in the filter, the size of the filter
    pub cells: usize,
    /// Number of hash functions
    pub hash_number: usize,
    /// Number of inserted values
    pub members: usize,
    /// Number of collisions occurred
    pub collisions: usize,
    /// Safeness probability over the entire filter
    pub safeness: f64,
    /// Number of areas
    pub area_number: usize,
    /// Number of members per area
    pub area_members: Vec<usize>,
    /// Expected number of cells for each area
    pub area_expected_cells: Vec<i64>,
    /// Number of cells occupied by each area
    pub area_cells: Vec<usize>,
    /// Number of collision of the same area value on the same cell
    pub area_self_collisions: Vec<usize>,
    /// Prior false positive probability for each area
    pub area_prior_fpp: Vec<f64>,
    /// Posterior false positive probability for each area
    pub area_fpp: Vec<f64>,
    /// Prior inter-set error probability for each area
    pub area_prior_isep: Vec<f64>,
    /// Posterior inter-set error probability for each area
    pub area_isep: Vec<f64>,
    /// Prior area-specific safeness probability
    pub area_prior_safep: Vec<f64>,
}

impl Metrics {
    /// Returns the number of inserted elements for the input area
    pub fn get_area_members(&self, index: usize) -> Option<usize> {
        self.area_members.get(index).cloned()
    }

    /// Returns the sparsity of the entire SBF
    pub fn get_filter_sparsity(&self) -> f64 {
        let sum: usize = self
            .area_cells
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
        p.pow(self.hash_number as f64)
    }

    /// Returns the expected emersion value for the input area
    pub fn get_expected_area_emersion(&self, area: usize) -> f64 {
        let cells_with_greater_area_index: usize =
            self.area_members.par_iter().skip(area).skip(1).sum();
        let p = 1.0f64 - 1.0f64 / self.cells as f64;
        p.pow(self.hash_number as f64 * cells_with_greater_area_index as f64)
    }

    /// Returns the emersion value for the input area
    pub fn get_area_emersion(&self, area: usize) -> Option<f64> {
        if self.area_cells[area] == 0 || self.hash_number == 0 {
            None
        } else {
            match (
                self.area_cells.get(area),
                self.area_members.get(area),
                self.area_self_collisions.get(area),
            ) {
                (Some(&area_cells), Some(&area_members), Some(&area_self_collisions)) => {
                    let a = area_cells as f64;
                    let b =
                        area_members as f64 * self.hash_number as f64 - area_self_collisions as f64;
                    Some(a / b)
                }
                _ => None,
            }
        }
    }

    /// Returns the prior false positive probability over the entire filter
    pub fn get_filter_prior_fpp(&self) -> f64 {
        let p = 1.0 - 1.0 / self.cells as f64;
        let p = 1.0 - p.powf(self.hash_number as f64 * self.members as f64);
        p.powf(self.hash_number as f64)
    }

    /// Computes posterior area-specific false positives probability (fpp)
    pub fn set_area_fpp(&mut self) {
        println!("AREA NUMBER: {}", self.area_number);
        (1..self.area_number).rev().for_each(|i| {
            let c: usize = (i..self.area_number).map(|j| self.area_cells[j]).sum();

            let p = c as f64 / self.cells as f64;
            let p = p.powi(self.hash_number as i32);

            self.area_fpp[i] = p;

            (i..self.area_number - 1).for_each(|j| {
                self.area_fpp[i] -= self.area_fpp[j + 1];
            });
            self.area_fpp[i] = self.area_fpp[i].max(0.0);
        })
    }

    /// Computes prior area-specific false positives probability (prior_fpp)
    pub fn set_prior_area_fpp(&mut self) {
        (1..self.area_number).rev().for_each(|i| {
            let c: usize = (i..self.area_number).map(|j| self.area_members[j]).sum();

            let p = 1.0 - 1.0 / self.cells as f64;
            let p = 1.0 - p.powi((self.hash_number * c) as i32);
            let p = p.powi(self.hash_number as i32);

            self.area_fpp[i] = p;

            (i..self.area_number - 1).for_each(|j| {
                self.area_prior_fpp[i] -= self.area_prior_fpp[j + 1];
            });
            self.area_prior_fpp[i] = self.area_prior_fpp[i].max(0.0);
        })
    }

    /// Computes posterior area-specific inter-set error probability (isep)
    pub fn set_area_isep(&mut self) {
        (1..self.area_number).rev().for_each(|i| {
            let p = 1.0 - self.get_area_emersion(i).unwrap_or(-1.0);
            let p = p.powi(self.hash_number as i32);

            self.area_isep[i] = p;
        })
    }

    /// Computes prior area-specific inter-set error probability (prior_isep),
    /// computes prior area-specific safeness probability (prior_safep) and
    /// the overall safeness probability for the entire filter (safeness)
    pub fn set_prior_area_isep(&mut self) {
        let mut p3 = 1.0;
        (1..self.area_number).rev().for_each(|i| {
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

    /// Computes the expected number of cells for each area (expected_cells)
    pub fn set_expected_area_cells(&mut self) {
        (1..self.area_number).rev().for_each(|i| {
            let n_fill: usize = (i..self.area_number).map(|j| self.area_members[j]).sum();

            let p1 = 1.0 - 1.0 / self.cells as f64;
            let p2 = p1.pow((self.hash_number * n_fill) as f64);
            self.area_expected_cells[i] = (self.cells as f64 * p1 * p2) as i64;
        })
    }
}
