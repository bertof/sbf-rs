extern crate byteorder;
extern crate md5;
extern crate rand;
extern crate serde;
extern crate serde_json;

use rand::Rng;
use std::{io, fs};
use std::io::{Write, LineWriter, Cursor};
use serde::{Deserialize, Serialize};
use byteorder::ReadBytesExt;
use std::collections::HashMap;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
struct HSBF {
    /// Number of unique values that can
    areas: u8,
    /// Salts used by the hash functions
    salts: Vec<Vec<u8>>,
    /// Max length of the names that can be inserted in the filter
    digest_length: u32,
    /// Data container
    map: Vec<u8>,
    /// Counter for each insert in each area
    stat_inserts: HashMap<u8, usize>,
}

impl HSBF {
    pub fn new(filter_size: usize, areas: u8, hashes: usize, name_length: u32) -> HSBF {
        let mut rng = rand::thread_rng();
        let hash_salts = (0..hashes).map(|_| {
            (0..name_length).map(|_| rng.gen()).collect::<Vec<u8>>()
        }).collect();

        HSBF {
            areas,
            salts: hash_salts,
            map: vec![0; filter_size],
            digest_length: name_length,
            stat_inserts: (0..areas).map(|v| (v, 0)).collect(),
        }
    }

    pub fn print_filter(&self) {
        println!("{:?}", &self.map)
    }

    pub fn get_hash_salts(&self) -> Vec<Vec<u8>> {
        self.salts.clone()
    }

    fn hash(buff: &Vec<u8>) -> Vec<u8> {
        md5::compute(buff).to_vec()
    }

    pub fn calc_indexes(&self, name: &str) -> Vec<usize> {
        self.salts.iter().map(|salt| {
            let buffer: Vec<u8> = name.as_bytes().iter().zip(salt)
                .map(|(&h, &v)| h ^ v).collect();

            let digest = HSBF::hash(&buffer).drain(0..8).collect::<Vec<u8>>();

            Cursor::new(digest).read_u64::<byteorder::NativeEndian>()
                .unwrap() as usize % self.map.len()
        }).collect()
    }

    pub fn insert(&mut self, name: &str, value: u8) {
        if value >= self.areas {
            panic!("Area index out of range")
        }
        *self.stat_inserts.get_mut(&value).unwrap() += 1;
        for index in self.calc_indexes(&name) {
            self.set_cell(index, value)
        }
    }

    pub fn check(&self, name: &str) -> u8 {
        self.calc_indexes(&name).iter()
            .map(|&ind| *self.get_cell(ind))
            .min().unwrap()
    }

    pub fn get_area_members(&self, value: u8) -> usize {
        if value >= self.areas {
            panic!("Area index out of range")
        }
        self.stat_inserts.get(&value).unwrap().clone()
    }

    pub fn write_hash_salts(&self, path: &str) -> io::Result<()> {
        let file = fs::File::create(path)?;
        let mut lw = LineWriter::new(file);

        for hash in &self.salts {
            let s_base64 = base64::encode(&hash) + "\n";
            lw.write_all(s_base64.as_bytes())?
        }

        Ok(())
    }

    pub fn read_hash_salts(&mut self, path: &str) -> io::Result<()> {
        let content = fs::read_to_string(path)?;
        let mut hash_salts = Vec::new();
        for line in content.trim().split("\n") {
            // Decode base64 to Vec<u8>
            let b_hash = base64::decode(line)
                .map_err(|_| io::Error::new(
                    io::ErrorKind::InvalidData, "Invalid format"))?;
            hash_salts.push(b_hash);
        }
        self.salts = hash_salts;

        Ok(())
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string(&self).expect("Can't encode the HSBF")
    }

    pub fn from_json(data: &String) -> io::Result<HSBF> {
        serde_json::from_str(data).map_err(|e| io::Error::new(
            io::ErrorKind::InvalidData, format!("Can't decode to a HSBF: {}", e)))
    }

    pub fn write_to_disk(&self, path: &str) -> io::Result<()> {
        let enc = self.to_json();
        let mut f = fs::File::create(path)?;
        f.write_all(enc.as_bytes())?;

        Ok(())
    }

    fn set_cell(&mut self, index: usize, value: u8) {
        if value < self.areas {
            *self.map.get_mut((index) as usize).expect("Filter index out of range") = value;
        } else {
            panic!("Area index out of range")
        }
    }

    fn get_cell(&self, index: usize) -> &u8 {
        return self.map.get(index).expect("Filter index out of range");
    }
}

#[cfg(test)]
mod tests {
    use crate::HSBF;
    use std::fs;
    use std::io::Read;

    #[test]
    fn test_set_cell() {
        let mut hsbf = HSBF::new(500, 10, 3, 50);
        hsbf.set_cell(0, 1);
        assert_eq!(hsbf.map[0], 1)
    }

    #[test]
    fn test_get_cell() {
        let mut hsbf = HSBF::new(500, 10, 3, 50);
        hsbf.set_cell(0, 1);
        assert_eq!(*hsbf.get_cell(0), 1)
    }

    #[test]
    #[should_panic]
    fn test_set_cell_index_out_of_range() {
        let mut hsbf = HSBF::new(500, 10, 3, 50);
        hsbf.set_cell(500, 0);
    }

    #[test]
    #[should_panic]
    fn test_set_cell_area_out_of_range() {
        let mut hsbf = HSBF::new(500, 10, 3, 50);
        hsbf.set_cell(0, 10);
    }

    #[test]
    fn test_print_filter() {
        let hsbf = HSBF::new(500, 10, 3, 50);
        hsbf.print_filter();
    }

    #[test]
    fn test_write_read_hash_salts() {
        let hsbf = HSBF::new(500, 10, 3, 50);
        let expected = hsbf.get_hash_salts();
        hsbf.write_hash_salts("/tmp/salts.txt").unwrap();

        let mut hsbf = HSBF::new(500, 10, 3, 50);
        hsbf.read_hash_salts("/tmp/salts.txt").unwrap();
        assert_eq!(hsbf.salts, expected)
    }

    #[test]
    fn test_write_to_disk() {
        let hsbf = HSBF::new(500, 10, 3, 50);
        hsbf.write_to_disk("/tmp/hsbf.json").unwrap();

        let mut f_content = String::new();
        fs::File::open("/tmp/hsbf.json").unwrap()
            .read_to_string(&mut f_content).unwrap();

        assert_eq!(hsbf, HSBF::from_json(&f_content).unwrap())
    }

    #[test]
    fn test_from_json() {
        let hsbf = HSBF::new(500, 10, 3, 50);
        hsbf.write_to_disk("/tmp/test_hsbf.json").unwrap();

        let mut f_content = String::new();
        fs::File::open("/tmp/test_hsbf.json").unwrap()
            .read_to_string(&mut f_content).unwrap();

        assert_eq!(f_content, hsbf.to_json())
    }

    #[test]
    fn test_insert_check() {
        let mut hsbf = HSBF::new(500, 10, 20, 50);
        hsbf.insert("TestInsert", 5);
        assert_eq!(hsbf.map.iter().filter(|&&v| v != 5 && v != 0).count(), 0);
        assert!(hsbf.map.iter().filter(|&&v| v == 5).count() > 0);
        assert_eq!(hsbf.get_area_members(5), 1);
        assert_eq!(hsbf.check("TestInsert"), 5)
    }
}
