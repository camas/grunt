use self::addon::Addon;
use self::curse::{CurseAPI, WOW_GAME_ID};
use self::lockfile::Lockfile;
use directories::ProjectDirs;
use fancy_regex::Regex;
use getset::{Getters, Setters};
use lazy_static::lazy_static;
use rayon::prelude::*;
use std::collections::{HashMap, HashSet, VecDeque};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

pub mod addon;
pub mod settings;

mod curse;
mod lockfile;
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

#[derive(Getters, Setters)]
#[getset(get = "pub", set = "pub")]
pub struct Grunt {
    is_new: bool,
    root_dir: PathBuf,
    lockfile_path: PathBuf,
    addons: Vec<Addon>,
    curse_api: Option<CurseAPI>,
}

impl Grunt {
    /// Create a new grunt instance from a given `AddOns` dir
    /// Reads data from `grunt.lockfile` if one exists
    pub fn new<P: AsRef<Path>>(path: P) -> Self {
        let path = path.as_ref();

        // Setup struct data
        let root_dir: PathBuf = std::fs::canonicalize(path).unwrap(); // Get absolute path
        let lockfile_path = root_dir.join("grunt.lockfile");
        let addons;
        let is_new;

        // Read lockfile if it exists
        if lockfile_path.exists() {
            is_new = true;
            let lockfile = Lockfile::from_file(&lockfile_path);
            addons = lockfile.addons.into_iter().map(Addon::from_info).collect();
        } else {
            is_new = false;
            addons = Vec::new();
        }

        // Return instance
        Grunt {
            root_dir,
            lockfile_path,
            is_new,
            addons,
            curse_api: None,
        }
    }

    /// Returns directories that aren't owned by any tracked addons
    pub fn find_untracked(&self) -> Vec<String> {
        // Get all directories in the root folder
        let all_dirs: Vec<String> = self
            .root_dir
            .read_dir()
            .unwrap()
            .filter_map(|entry| {
                let entry = entry.unwrap();
                if entry.file_type().unwrap().is_dir() {
                    Some(entry.file_name().to_str().unwrap().to_string())
                } else {
                    None
                }
            })
            .collect();
        // Get all directories owned by addons
        let all_tracked: Vec<&String> = self.addons.iter().flat_map(|addon| addon.dirs()).collect();
        // Return directories not owned by addons
        all_dirs
            .into_iter()
            .filter(|dir| !all_tracked.contains(&dir))
            .collect()
    }

    /// Attempts to resolve untracked addons
    /// Adds any found to the lockfile
    /// Returns a vec of references to the addons found
    pub fn resolve<F>(&mut self, mut prog: F)
    where
        F: FnMut(ResolveProgress),
    {
        let untracked = self.find_untracked();
        let mut new_addons = Vec::new();

        // Get addon information from `{Addon}.toc` if it is there
        let tukui_id_string = "## X-Tukui-ProjectID:";
        let tukui_project_string = "## X-Tukui-ProjectFolders:";
        for dir in &untracked {
            // Get the path to the .toc for each addon
            let toc = self.root_dir.join(&dir).join(format!("{}.toc", dir));
            if !toc.exists() {
                panic!("{}.toc not found", dir);
            }

            // Open file for reading
            let file = File::open(toc).expect("Error opening .toc file");
            let reader = BufReader::new(file);

            // Loop through every line checking for relevant ones
            let mut tukui_id = None;
            let mut tukui_dirs = None;
            for line in reader.lines() {
                let line = line.expect("Error reading .toc");
                if line.starts_with(tukui_id_string) {
                    tukui_id = Some(
                        line[tukui_id_string.len()..]
                            .trim()
                            .parse::<i64>()
                            .expect("Error parsing Tukui ID"),
                    );
                } else if line.starts_with(tukui_project_string) {
                    tukui_dirs = Some(
                        line[tukui_project_string.len()..]
                            .split(',')
                            .map(|s| s.trim().to_string())
                            .collect::<Vec<String>>(),
                    );
                }
            }

            // Check if tukui info found
            if let Some(tukui_id) = tukui_id {
                if let Some(tukui_dirs) = tukui_dirs {
                    let addon = Addon::from_tukui_info(dir.clone(), tukui_id, tukui_dirs);
                    prog(ResolveProgress::NewAddon {
                        name: dir.clone(),
                        desc: addon.desc_string(),
                    });
                    new_addons.push(addon);
                } else {
                    panic!("X-Tukui-ProjectID found but no X-Tukui-ProjectFolders");
                }
            }
        }
        self.addons.extend(new_addons);
        let untracked = self.find_untracked();

        // Curse
        let curse_addons = self.resolve_curse(untracked);
        for addon in curse_addons.iter() {
            prog(ResolveProgress::NewAddon {
                name: addon.name().clone(),
                desc: addon.desc_string(),
            })
        }
        self.addons.extend(curse_addons);

        // Finish
        prog(ResolveProgress::Finished {
            not_found: self.find_untracked(),
        });
    }

    /// Save the lockfile
    pub fn save_lockfile(&self) {
        Lockfile::from_grunt(self).save(&self.lockfile_path);
    }

    /// Updates addons
    pub fn update_addons(&mut self) {
        todo!();
    }

    /// Check that two addons don't claim the same directory
    pub fn check_conflicts(&self) {
        todo!();
    }

    pub fn get_addon(&self, name: &str) -> Option<&Addon> {
        self.addons.iter().find(|addon| addon.name() == name)
    }

    /// Removes all the addons with the specified names
    /// Panics if an addon not found
    pub fn remove_addons(&mut self, names: &[String]) {
        for name in names {
            let addon_index = self
                .addons
                .iter()
                .position(|addon| addon.name() == name)
                .unwrap_or_else(|| panic!("Couldn't find addon {}", name));
            let addon = self.addons.remove(addon_index);
            addon.dirs().iter().for_each(|dir| {
                std::fs::remove_dir_all(self.root_dir.join(dir)).expect("Error deleting addon dir");
            })
        }
    }

    /// Initializes the curse api if not initialized and returns it
    fn get_api(&mut self) -> &CurseAPI {
        if self.curse_api.is_none() {
            self.curse_api = Some(CurseAPI::init());
        }
        self.curse_api.as_ref().unwrap()
    }

    fn resolve_curse(&mut self, untracked: Vec<String>) -> Vec<Addon> {
        // Get curse info for WoW
        let game_info = self.get_api().get_game_info(WOW_GAME_ID);

        // Compile regexes
        let addon_cat = &game_info.category_sections[0];
        // Check category is correct
        assert_eq!(addon_cat.name, "Addons");
        assert_eq!(addon_cat.package_type, 1);
        let initial_inclusion_regex = Regex::new(&addon_cat.initial_inclusion_pattern)
            .expect("Error compiling inclusion regex");
        let extra_inclusion_regex = Regex::new(&addon_cat.extra_include_pattern)
            .expect("Error compiling extra inclusion regex");
        let file_parsing_regex: HashMap<String, (regex::Regex, Regex)> = game_info
            .file_parsing_rules
            .iter()
            .map(|data| {
                let comment_strip_regex = regex::Regex::new(&data.comment_strip_pattern)
                    .expect("Error compiling comment strip regex");
                let inclusion_regex =
                    Regex::new(&data.inclusion_pattern).expect("Error compiling inclusion pattern");
                (
                    data.file_extension.clone(),
                    (comment_strip_regex, inclusion_regex),
                )
            })
            .collect();

        // Fingerprint each untracked dir
        let mut fingerprints: Vec<u32> = Vec::with_capacity(untracked.len());
        untracked
            .par_iter() // Easy parallelization
            .map(|dir_name| {
                let addon_dir = self.root_dir.join(dir_name);
                let mut to_fingerprint = HashSet::new();
                let mut to_parse = VecDeque::new();

                // Add initial files
                let glob_pattern = format!("{}/**/*.*", addon_dir.to_str().unwrap());
                for path in glob::glob(&glob_pattern).expect("Glob pattern error") {
                    let path = path.expect("Glob error");
                    if !path.is_file() {
                        continue;
                    }

                    // Test relative path matches regexes
                    let relative_path = path
                        .strip_prefix(&self.root_dir)
                        .unwrap()
                        .to_str()
                        .unwrap()
                        .to_ascii_lowercase()
                        .replace("/", "\\"); // Convert to windows seperator
                    if initial_inclusion_regex.is_match(&relative_path).unwrap() {
                        to_parse.push_back(path);
                    } else if extra_inclusion_regex.is_match(&relative_path).unwrap() {
                        to_fingerprint.insert(path);
                    }
                }

                // Parse additional files
                while let Some(path) = to_parse.pop_front() {
                    if !path.exists() || !path.is_file() {
                        panic!("Invalid file given to parse");
                    }

                    to_fingerprint.insert(path.clone());

                    // Skip if no rules for extension
                    let ext = format!(".{}", path.extension().unwrap().to_str().unwrap());
                    if !file_parsing_regex.contains_key(&ext) {
                        continue;
                    }

                    // Parse file for matches
                    // TODO: Parse line by line because regex is \n sensitive
                    let (comment_strip_regex, inclusion_regex) =
                        file_parsing_regex.get(&ext).unwrap();
                    let text = std::fs::read_to_string(&path).expect("Error reading file");
                    let text = comment_strip_regex.replace_all(&text, "");
                    for line in text.split(&['\n', '\r'][..]) {
                        let mut last_offset = 0;
                        while let Some(inc_match) = inclusion_regex
                            .captures_from_pos(line, last_offset)
                            .unwrap()
                        {
                            last_offset = inc_match.get(0).unwrap().end();
                            let path_match = inc_match.get(1).unwrap().as_str();
                            // Path might be case insensitive and have windows separators. Find it
                            let path_match = path_match.replace("\\", "/");
                            let parent = path.parent().unwrap();
                            let real_path = find_file(parent.join(Path::new(&path_match)));
                            to_parse.push_back(real_path);
                        }
                    }
                }

                // Calculate fingerprints
                let mut fingerprints: Vec<u32> = to_fingerprint
                    .iter()
                    .map(|path| {
                        // Read file, removing whitespace
                        let data: Vec<u8> = std::fs::read(path)
                            .expect("Error reading file for fingerprinting")
                            .into_iter()
                            .filter(|&b| b != b' ' && b != b'\n' && b != b'\r' && b != b'\t')
                            .collect();
                        murmur2::calculate_hash(&data, 1)
                    })
                    .collect();

                // Calculate overall fingerprint
                fingerprints.sort();
                let to_hash = fingerprints
                    .iter()
                    .map(|val| val.to_string())
                    .collect::<Vec<String>>()
                    .join("");
                murmur2::calculate_hash(to_hash.as_bytes(), 1)
            })
            .collect_into_vec(&mut fingerprints);

        // Query api for fingerprint matches
        let results = self.get_api().fingerprint_search(&fingerprints);

        results
            .exact_matches
            .iter()
            .map(|mat| {
                let index = fingerprints
                    .iter()
                    // Assumes last module is the main one
                    .position(|&x| x == mat.file.modules.last().unwrap().fingerprint)
                    .unwrap();
                let name = untracked[index].clone();
                Addon::from_curse_info(name, mat)
            })
            .collect()
    }
}

pub enum ResolveProgress {
    NewAddon { name: String, desc: String },
    Finished { not_found: Vec<String> },
}

/// Finds a case sensitive path from an insensitive path
/// Useful if, say, a WoW addon points to a local path in a different case but you're not on Windows
fn find_file<P>(path: P) -> PathBuf
where
    P: AsRef<Path>,
{
    let mut current = path.as_ref();
    let mut to_finds = Vec::new();

    // Find first parent that exists
    while !current.exists() {
        to_finds.push(current.file_name().unwrap());
        current = current.parent().unwrap();
    }

    // Match to finds
    let mut current = current.to_path_buf();
    to_finds.reverse();
    for to_find in to_finds {
        let mut children = current.read_dir().unwrap();
        let lower = to_find.to_str().unwrap().to_ascii_lowercase();
        let found = children
            .find(|x| {
                x.as_ref()
                    .unwrap()
                    .file_name()
                    .to_str()
                    .unwrap()
                    .to_ascii_lowercase()
                    == lower
            })
            .unwrap()
            .unwrap();
        current = found.path();
    }
    current
}
