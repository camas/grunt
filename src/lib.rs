use self::addon::{Addon, AddonType};
use self::curse::{CurseAPI, WOW_GAME_ID};
use self::lockfile::Lockfile;
use fancy_regex::Regex;
use getset::{Getters, Setters};
use rayon::prelude::*;
use std::collections::{HashMap, HashSet, VecDeque};
use std::fs::File;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::thread;

pub mod addon;
pub mod settings;

mod curse;
mod lockfile;
mod murmur2;
mod tsm;
mod tukui;

#[derive(Getters, Setters)]
#[getset(get = "pub", set = "pub")]
pub struct Grunt {
    is_new: bool,
    root_dir: PathBuf,
    lockfile_path: PathBuf,
    addons: Vec<Addon>,
    curse_api: CurseAPI,
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
            curse_api: CurseAPI::init(),
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
    /// Progress is reported using `prog`
    pub fn resolve<F>(&mut self, mut prog: F)
    where
        F: FnMut(ResolveProgress),
    {
        let untracked = self.find_untracked();
        let mut new_addons = Vec::new();

        // Check for TSM addons
        let tsm_string = "TradeSkillMaster";
        let tsm_dir = self.root_dir.join(tsm_string);
        if untracked.contains(&tsm_string.to_string()) && tsm_dir.exists() {
            let version = get_toc_version(tsm_dir.join("TradeSkillMaster.toc"));
            let tsm_addon = Addon::init_tsm(version);
            prog(ResolveProgress::NewAddon {
                name: tsm_string.to_string(),
                desc: tsm_addon.desc_string(),
            });
            self.addons.push(tsm_addon);
        }
        let tsm_helper_string = "TradeSkillMaster_AppHelper";
        let tsm_helper_dir = self.root_dir.join(tsm_helper_string);
        if untracked.contains(&tsm_helper_string.to_string()) && tsm_helper_dir.exists() {
            let version = get_toc_version(tsm_helper_dir.join("TradeSkillMaster_AppHelper.toc"));
            let tsm_helper_addon = Addon::init_tsm_helper(version);
            prog(ResolveProgress::NewAddon {
                name: tsm_helper_string.to_string(),
                desc: tsm_helper_addon.desc_string(),
            });
            self.addons.push(tsm_helper_addon);
        }
        let untracked = self.find_untracked();

        // Get addon information from `{Addon}.toc` if it is there
        let tukui_id_string = "## X-Tukui-ProjectID:";
        let tukui_project_string = "## X-Tukui-ProjectFolders:";
        let version_string = "## Version:";
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
            let mut version = None;
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
                } else if line.starts_with(version_string) {
                    version = Some(line[version_string.len()..].trim().to_string())
                }
            }

            // Check if tukui info found
            if let Some(tukui_id) = tukui_id {
                if let Some(tukui_dirs) = tukui_dirs {
                    if let Some(version) = version {
                        let addon =
                            Addon::from_tukui_info(dir.clone(), tukui_id, tukui_dirs, version);
                        prog(ResolveProgress::NewAddon {
                            name: dir.clone(),
                            desc: addon.desc_string(),
                        });
                        new_addons.push(addon);
                    } else {
                        panic!("Missing addon version!");
                    }
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
    pub fn update_addons<F>(&mut self, mut check_update: F)
    where
        F: FnMut(Vec<Updateable>) -> Vec<Updateable>,
    {
        // Get information from addon list needed to download update information
        // Curse IDs
        let curse_ids: Vec<(String, i64)> = self
            .addons
            .iter()
            .filter(|addon| addon.addon_type() == &AddonType::Curse)
            .map(|addon| (addon.addon_id().clone(), addon.version().parse().unwrap()))
            .collect();
        // Tukui IDs
        let tukui_ids: Vec<String> = self
            .addons
            .iter()
            .filter(|addon| addon.addon_type() == &AddonType::Tukui && addon.addon_id() != "-2")
            .map(|addon| addon.addon_id().clone())
            .collect();
        // Get ElvUI addon if it exists. (Tukui special case)
        let has_elvui_addon = self
            .addons
            .iter()
            .any(|addon| addon.addon_type() == &AddonType::Tukui && addon.addon_id() == "-2");

        // Create threads to download info for each set of IDs
        // Curse
        // Returns a vec of (curse id, latest id, download url)
        let curse_thread = thread::spawn(move || {
            // Return early if no curse addons
            if curse_ids.is_empty() {
                return HashMap::new();
            }
            let mut to_update = HashMap::new();
            let api = CurseAPI::init(); // Bit of a hack
            let ids: Vec<&String> = curse_ids.iter().map(|(id, _)| id).collect();
            let addon_infos = api.get_addons_info(&ids);
            for info in addon_infos {
                // Get the latest version by selecting the file with the highest id (newest)
                let latest = info
                    .latest_files
                    .iter()
                    // Only look at retail files
                    .filter(|file| file.game_version_flavor == "wow_retail")
                    .max_by(|file_a, &file_b| file_a.id.cmp(&file_b.id))
                    .unwrap();
                let (curse_id, _) = curse_ids
                    .iter()
                    .find(|(id, _)| id == &info.id.to_string())
                    .unwrap();
                to_update.insert(curse_id.clone(), (latest.id, latest.download_url.clone()));
            }
            to_update
        });
        // Tukui
        let tukui_thread = thread::spawn(move || {
            if tukui_ids.is_empty() {
                return HashMap::new();
            }
            let tukui_infos = tukui::get_addon_infos();
            let mut map = HashMap::new();
            for id in tukui_ids {
                let info = tukui_infos
                    .iter()
                    .find(|info| info.id == id)
                    .expect("No tukui addon with the right ID found");
                map.insert(id, (info.version.clone(), info.url.clone()));
            }
            map
        });
        // ElvUI special case
        let elvui_thread = thread::spawn(move || {
            if !has_elvui_addon {
                return ("".to_string(), "".to_string());
            }
            let elvui_info = tukui::get_elvui_info();
            (elvui_info.version, elvui_info.url)
        });

        // Wait for threads to finish
        let mut latest_curse = curse_thread.join().unwrap();
        let mut latest_tukui = tukui_thread.join().unwrap();
        let elvui_info = elvui_thread.join().unwrap();

        // Find out which addons need updating
        let outdated = self
            .addons
            .iter()
            .enumerate()
            .filter_map(|(index, addon)| {
                let data = match addon.addon_type() {
                    AddonType::Curse => {
                        let current: i64 = addon.version().parse().unwrap();
                        let (latest, url) = latest_curse.remove(addon.addon_id()).unwrap();
                        if latest > current {
                            Some((latest.to_string(), url))
                        } else {
                            None
                        }
                    }
                    AddonType::Tukui => {
                        let curr = addon.version();
                        let (latest, url) = if addon.addon_id() == "-2" {
                            elvui_info.clone()
                        } else {
                            latest_tukui.remove(addon.addon_id()).unwrap()
                        };

                        if &latest > curr {
                            Some((latest, url))
                        } else {
                            None
                        }
                    }
                    _ => None,
                    //_ => panic!("Unknown addon type"),
                };
                if let Some((version, url)) = data {
                    Some(Updateable {
                        index,
                        name: addon.name().clone(),
                        new_version: version,
                        url,
                    })
                } else {
                    None
                }
            })
            .collect();
        // let info = Updateable {
        //     index,
        //     name: addon.name().clone(),
        //     new_version: latest_str,
        //     url,
        // };
        // outdated.push(info);

        // Ask user
        let outdated = check_update(outdated);

        // Download/unpack updates
        let tmp_dir = tempfile::Builder::new().prefix("grunt").tempdir().unwrap();
        outdated.par_iter().for_each(|upd| {
            // Download to temp file
            let download_loc = tmp_dir.path().join(format!("update{}.download", upd.index));
            let mut file = File::create(&download_loc).unwrap();
            let mut resp = reqwest::blocking::get(&upd.url).expect("Error downloading update");
            std::io::copy(&mut resp, &mut file).expect("Error downloading update to temp file");
            // Explicity close file
            drop(file);

            // Unzip downloaded file to temp dir
            let unzip_dir = tmp_dir.path().join(format!("unpacked{}", upd.index));
            std::fs::create_dir(&unzip_dir).unwrap();
            let file = File::open(&download_loc).unwrap();
            let reader = BufReader::new(file);
            let mut zip = zip::ZipArchive::new(reader).expect("Error reading zip");
            // Iterate through each entry in the zip
            for i in 0..zip.len() {
                let mut entry = zip.by_index(i).unwrap();
                let entry_path = entry.sanitized_name();
                let out_path = unzip_dir.join(entry_path);
                // Create parent dir
                std::fs::create_dir_all(out_path.parent().unwrap()).unwrap();
                if entry.is_dir() {
                    // Create empty dir
                    std::fs::create_dir(&out_path).unwrap();
                } else {
                    // Extract file
                    let mut out_file = File::create(&out_path).unwrap();
                    std::io::copy(&mut entry, &mut out_file).expect("Error extracting from zip");
                }
            }
        });

        // Check for dir conflicts then replace addon files
        // First get all directory categories
        let outdated_addons: Vec<&Addon> = outdated
            .iter()
            .map(|upd| self.addons.get(upd.index).unwrap())
            .collect();
        let dirs_to_remove: Vec<&String> = outdated_addons
            .iter()
            .flat_map(|addon| addon.dirs())
            .collect();
        let outdated_indexes: Vec<usize> = outdated.iter().map(|upd| upd.index).collect();
        let untouched_dirs: Vec<&String> = self
            .addons
            .iter()
            .enumerate()
            .filter(|(index, _)| !outdated_indexes.contains(index))
            .flat_map(|(_, addon)| addon.dirs())
            .collect();
        let new_dirs: Vec<String> = outdated_indexes
            .iter()
            .flat_map(|index| {
                // Read all entries in unpack directory
                let unpack_dir = tmp_dir.path().join(format!("unpacked{}", index));
                std::fs::read_dir(&unpack_dir)
                    .unwrap()
                    .map(|entry| {
                        let entry = entry.unwrap();
                        // Panic if file
                        if entry.path().is_file() {
                            panic!("File found. Only directories expected in addon update zip");
                        }
                        entry.file_name().to_str().unwrap().to_string()
                    })
                    .collect::<Vec<String>>()
            })
            .collect();
        // Check new dirs for duplicates
        for (index, dir) in new_dirs.iter().enumerate() {
            for other in new_dirs.iter().skip(index + 1) {
                if dir == other {
                    panic!("Dir conflict");
                }
            }
        }
        // Check new and unchanged dirs for conflicts
        for dir in new_dirs.iter() {
            for other in untouched_dirs.iter() {
                if &dir == other {
                    panic!("Dir conflict");
                }
            }
        }
        // Delete old dirs
        for dir_name in dirs_to_remove.iter() {
            let path = self.root_dir.join(dir_name);
            if path.exists() {
                std::fs::remove_dir_all(path).expect("Error deleting outdated addon");
            }
        }
        // Copy new ones
        for index in outdated_indexes.iter() {
            let unpacked_dir = tmp_dir.path().join(format!("unpacked{}", index));
            for entry in walkdir::WalkDir::new(&unpacked_dir) {
                let entry = entry.unwrap();
                let relative_path = entry.path().strip_prefix(&unpacked_dir).unwrap();
                let new_path = self.root_dir.join(relative_path);
                if entry.path().is_dir() {
                    std::fs::create_dir_all(new_path).unwrap();
                } else {
                    std::fs::create_dir_all(new_path.parent().unwrap()).unwrap();
                    let mut reader = File::open(entry.path()).unwrap();
                    let mut writer = File::create(new_path).unwrap();
                    std::io::copy(&mut reader, &mut writer).expect("Error copying new addon files");
                }
            }
        }

        // Update addon data including updating the dirs
        for upd in outdated.into_iter() {
            let addon = self.addons.get_mut(upd.index).unwrap();
            let unpacked_dir = tmp_dir.path().join(format!("unpacked{}", upd.index));
            let new_dirs = unpacked_dir
                .read_dir()
                .unwrap()
                .map(|entry| entry.unwrap())
                .filter(|entry| entry.path().is_dir())
                .map(|entry| entry.file_name().to_str().unwrap().to_string())
                .collect::<Vec<String>>();
            addon.set_dirs(new_dirs);
            addon.set_version(upd.new_version);
        }
    }

    /// Check that two addons don't claim the same directory
    pub fn check_conflicts(&self) -> Vec<Conflict> {
        let mut conflicts = Vec::new();
        for (i, addon) in self.addons.iter().enumerate() {
            for (j, other) in self.addons.iter().enumerate().skip(i + 1) {
                // Check no match between dirs
                for dir in addon.dirs() {
                    if other.dirs().contains(dir) {
                        let conflict = Conflict {
                            addon_a_index: i,
                            addon_b_index: j,
                            dir: dir.clone(),
                        };
                        conflicts.push(conflict);
                    }
                }
            }
        }
        conflicts
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

    /// Deletes top-level directories and their contents if they are untracked
    pub fn remove_dirs(&self, dirs: Vec<String>) {
        let untracked = self.find_untracked();
        let root = self.root_dir();
        for dir in dirs {
            if !untracked.contains(&dir) {
                panic!("{} is a tracked directory", dir);
            }
            let path = root.join(dir);
            std::fs::remove_dir_all(path).expect("Error deleting the contents of ");
        }
    }

    /// Updates the data in TradeSkillMaster_AppHelper by using the (undocumented) tsm api
    pub fn update_tsm_data(&self, tsm_email: &str, tsm_pass: &str) {
        // Get TSM AppHelper addon
        let addon = self
            .addons
            .iter()
            .find(|a| a.name() == "TradeSkillMaster_AppHelper")
            .expect("TSM AppHelper not found");

        // Read current data
        let mut current_data: HashMap<(String, String), (String, u64)> = HashMap::new();
        let path = self.root_dir.join(addon.name()).join("AppData.lua");
        let f = File::open(&path).unwrap();
        for line in BufReader::new(f).lines() {
            // Each line is of the format
            // `{data} --<{data_type},{realm},{time}>`
            let line = line.unwrap();
            let mut split = line.split("--");
            let data = split.next().unwrap().trim_end_matches(' ').into();
            let comment_data = split
                .next()
                .unwrap()
                .trim_start_matches('<')
                .trim_end_matches('>');
            let mut comment_split = comment_data.split(',');
            let data_type = comment_split.next().unwrap().into();
            let realm = comment_split.next().unwrap().into();
            let time: u64 = comment_split.next().unwrap().parse().unwrap();
            current_data.insert((data_type, realm), (data, time));
        }

        // Login to the tsm api
        let mut api = tsm::TSMApi::new();
        api.login(tsm_email, tsm_pass);
        let status = api.get_status();

        // Update to latest data
        let time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let addon_message_str = format!(
            "{{id={},msg=\"{}\"}}",
            status.addon_message.id, status.addon_message.msg
        );
        let new_data = format!(
            "{{version={},lastSync={},message={},news={}}}",
            tsm::APP_VERSION,
            time,
            addon_message_str,
            status.addon_news
        );
        current_data.insert(("APP_INFO".into(), "Global".into()), (new_data, time));
        for region in status.regions {
            let data = api.auctiondb("region", region.id);
            current_data.insert(
                ("AUCTIONDB_MARKET_DATA".into(), region.name.clone()),
                (data, region.last_modified),
            );
        }
        for realm in status.realms {
            let data = api.auctiondb("realm", realm.master_id);
            current_data.insert(
                ("AUCTIONDB_MARKET_DATA".into(), realm.name.clone()),
                (data, realm.last_modified),
            );
        }

        // Save
        let mut f = File::create(&path).unwrap();
        for ((data_type, data_name), (data, time)) in current_data.iter() {
            let line = format!(
                "select(2, ...).LoadData(\"{}\",\"{}\",[[return {}]]) --<{},{},{}>\r\n",
                data_type, data_name, data, data_type, data_name, time
            );
            f.write_all(line.as_bytes()).unwrap();
        }
    }

    fn resolve_curse(&mut self, untracked: Vec<String>) -> Vec<Addon> {
        // Get curse info for WoW
        let game_info = self.curse_api.get_game_info(WOW_GAME_ID);

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
        let results = self.curse_api.fingerprint_search(&fingerprints);

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

pub struct Updateable {
    pub index: usize,
    pub name: String,
    pub new_version: String,
    pub url: String,
}

pub struct Conflict {
    pub addon_a_index: usize,
    pub addon_b_index: usize,
    pub dir: String,
}

pub enum ResolveProgress {
    NewAddon { name: String, desc: String },
    Finished { not_found: Vec<String> },
}

/// Get the version string from a `.toc` file
fn get_toc_version<P>(path: P) -> String
where
    P: AsRef<Path>,
{
    let version_string = "## Version:";
    let file = File::open(path).expect("Error opening .toc file");
    let reader = BufReader::new(file);
    for line in reader.lines() {
        let line = line.unwrap();
        if line.starts_with(version_string) {
            return line[version_string.len()..].trim().to_string();
        }
    }
    panic!("Couldn't find toc version");
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
