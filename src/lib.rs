use dialoguer;
use directories::ProjectDirs;
use lazy_static::lazy_static;
use log::*;
use simplelog::*;
use std::fs::File;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

pub mod settings;

mod murmur2;

lazy_static! {
    static ref PROJECT_DIRS: ProjectDirs = {
        let dirs = ProjectDirs::from("", "", "grunt").expect("Couldn't find project dirs");
        std::fs::create_dir_all(dirs.data_dir()).expect("Couldn't create data directory");
        dirs
    };
}

pub fn get_project_dirs() -> &'static ProjectDirs {
    &PROJECT_DIRS
}

pub struct Grunt {}

impl Grunt {}

/// Initialize logging to output and optionally a file
pub fn init_logging(stdout_verbosity: u8, log_file: Option<(PathBuf, u8)>) {
    fn get_log_level(verbosity: u8) -> LevelFilter {
        match verbosity {
            0 => LevelFilter::Off,
            1 => LevelFilter::Error,
            2 => LevelFilter::Warn,
            3 => LevelFilter::Info,
            4 => LevelFilter::Debug,
            5 => LevelFilter::Trace,
            _ => panic!("Invalid value {} given for verbosity", verbosity),
        }
    }

    let mut loggers: Vec<Box<dyn SharedLogger>> = vec![TermLogger::new(
        get_log_level(stdout_verbosity),
        Config::default(),
        TerminalMode::Mixed,
    )
    .unwrap()];
    if let Some((path, v)) = log_file {
        debug!("Opening '{}' for logging", path.to_string_lossy());
        let file = File::create(&path).expect("Couldn't open log file for writing");
        let write_logger = WriteLogger::new(get_log_level(v), Config::default(), file);
        loggers.push(write_logger);
    }
    CombinedLogger::init(loggers).expect("Couldn't initialize logging");
    info!("Logging initialized");
}
