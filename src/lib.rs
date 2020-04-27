use dialoguer;
use directories::ProjectDirs;
use lazy_static::lazy_static;
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
