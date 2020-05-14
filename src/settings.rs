use getset::{Getters, Setters};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::path::Path;

static CURRENT_VERSION: u32 = 1;

#[derive(Serialize, Deserialize, Getters, Setters)]
#[getset(get = "pub", set = "pub")]
pub struct Settings {
    version: u32,
    default_dir: Option<String>,
    tsm_email: Option<String>,
    tsm_pass: Option<String>,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            version: CURRENT_VERSION,
            default_dir: None,
            tsm_email: None,
            tsm_pass: None,
        }
    }
}

impl Settings {
    /// Uses the default settings
    pub fn new() -> Self {
        Default::default()
    }

    /// Loads settings from a file
    pub fn from_file<P: AsRef<Path>>(path: P) -> Self {
        let file = File::open(path).expect("Error opening settings file");
        let reader = std::io::BufReader::new(file);
        serde_json::from_reader::<_, Settings>(reader).expect("Error reading settings as json")
    }

    /// Loads settings from a file if it exists or uses default values
    pub fn from_file_or_new<P: AsRef<Path>>(path: P) -> Self {
        let path = path.as_ref();
        if path.exists() {
            Self::from_file(path)
        } else {
            Self::new()
        }
    }

    pub fn save<P: AsRef<Path>>(&self, path: P) {
        let file = File::create(path).expect("Error creating settings file");
        let writer = std::io::BufWriter::new(file);
        serde_json::to_writer_pretty(writer, self).expect("Error writing settings");
    }
}
