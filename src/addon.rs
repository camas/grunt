use crate::curse;
use crate::lockfile::AddonInfo;
use getset::{Getters, Setters};

#[derive(PartialEq, Getters, Setters)]
#[getset(get = "pub", set = "pub")]
pub struct Addon {
    name: String,
    addon_type: AddonType,
    addon_id: String,
    dirs: Vec<String>,
}

impl Addon {
    pub fn from_info(info: AddonInfo) -> Self {
        Addon {
            name: info.name,
            addon_type: info.addon_type,
            addon_id: info.addon_id,
            dirs: info.dirs,
        }
    }

    pub fn to_info(&self) -> AddonInfo {
        AddonInfo {
            name: self.name.clone(),
            addon_type: self.addon_type.clone(),
            addon_id: self.addon_id.clone(),
            dirs: self.dirs.clone(),
        }
    }

    pub fn from_curse_info(dir_name: String, info: &curse::AddonInfo) -> Self {
        let dirs = info
            .file
            .modules
            .iter()
            .map(|module| module.foldername.clone())
            .collect();
        Addon {
            name: dir_name,
            addon_type: AddonType::Curse,
            addon_id: info.id.to_string(),
            dirs,
        }
    }

    pub fn from_tukui_info(name: String, id: i64, dirs: Vec<String>) -> Self {
        Addon {
            name,
            addon_type: AddonType::Tukui,
            addon_id: id.to_string(),
            dirs,
        }
    }

    pub fn desc_string(&self) -> String {
        format!("{:?}:{}", self.addon_type, self.addon_id)
    }
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
pub enum AddonType {
    Unknown,
    Curse,
    Tukui,
}
