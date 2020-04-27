use std::path::Path;

pub struct Settings {}

impl Default for Settings {
    fn default() -> Self {
        todo!();
    }
}

impl Settings {
    /// Uses the default settings
    pub fn new() -> Self {
        Default::default()
    }

    /// Loads settings from a file
    pub fn from_file<P: AsRef<Path>>(path: P) -> Self {
        todo!();
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

    pub fn save<P: AsRef<Path>>(path: P) {
        todo!();
    }
}
