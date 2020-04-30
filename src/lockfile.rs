use crate::addon::AddonType;
use crate::Grunt;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::path::Path;

#[derive(Serialize, Deserialize)]
pub struct Lockfile {
    pub addons: Vec<AddonInfo>,
}

impl Lockfile {
    /// Initialize using data from the specified file
    pub fn from_file<P: AsRef<Path>>(path: P) -> Self {
        let file = File::open(path).expect("Error opening lockfile");
        let reader = BufReader::new(file);
        serde_json::from_reader(reader).expect("Error reading lockfile")
    }

    pub fn from_grunt(grunt: &Grunt) -> Self {
        let addons = grunt.addons.iter().map(|addon| addon.to_info()).collect();
        Lockfile { addons }
    }

    pub fn save<P: AsRef<Path>>(&self, path: P) {
        let file = File::create(path).expect("Error opening lockfile for write");
        let writer = BufWriter::new(file);
        serde_json::to_writer_pretty(writer, self).expect("Error writing to lockfile");
    }
}

#[derive(Serialize, Deserialize)]
pub struct AddonInfo {
    pub name: String,
    pub addon_type: AddonType,
    pub addon_id: String,
    pub dirs: Vec<String>,
}
