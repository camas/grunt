use crate::curse;
use crate::lockfile::AddonInfo;
use getset::{Getters, Setters};

#[derive(PartialEq, Getters, Setters)]
#[getset(get = "pub", set = "pub")]
pub struct Addon {
    name: String,
    addon_type: AddonType,
    addon_id: String,
    /// Internal string used to check for updates
    version: String,
    dirs: Vec<String>,
}

impl Addon {
    /// Initialize using the information from an `AddonInfo`
    pub fn from_info(info: AddonInfo) -> Self {
        Addon {
            name: info.name,
            addon_type: info.addon_type,
            addon_id: info.addon_id,
            version: info.version,
            dirs: info.dirs,
        }
    }

    /// Create an `AddonInfo` using this addon's info
    pub fn to_info(&self) -> AddonInfo {
        AddonInfo {
            name: self.name.clone(),
            addon_type: self.addon_type.clone(),
            addon_id: self.addon_id.clone(),
            version: self.version.clone(),
            dirs: self.dirs.clone(),
        }
    }

    /// Initialize a Curse addon using the information from a curse api response
    pub fn from_curse_info(dir_name: String, info: &curse::AddonFingerprintInfo) -> Self {
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
            version: info.file.id.to_string(),
            dirs,
        }
    }

    /// Initialize a tukui addon using the provided `id` and `dirs`
    pub fn from_tukui_info(name: String, id: i64, dirs: Vec<String>, version: String) -> Self {
        Addon {
            name,
            addon_type: AddonType::Tukui,
            addon_id: id.to_string(),
            version,
            dirs,
        }
    }

    /// Initialize using default values for addon `TradeSkillMaster`
    pub fn init_tsm(version: String) -> Self {
        let tsm_string = "TradeSkillMaster";
        Addon {
            name: tsm_string.to_string(),
            addon_type: AddonType::TSM,
            addon_id: "TradeSkillMaster".to_string(),
            version,
            dirs: vec![tsm_string.to_string()],
        }
    }

    /// Initialize using default values for addon `TradeSkillMaster_AppHelper`
    pub fn init_tsm_helper(version: String) -> Self {
        let tsm_helper_string = "TradeSkillMaster_AppHelper";
        Addon {
            name: tsm_helper_string.to_string(),
            addon_type: AddonType::TSM,
            addon_id: "AppHelper".to_string(),
            version,
            dirs: vec![tsm_helper_string.to_string()],
        }
    }

    /// Returns a short type:id string
    pub fn desc_string(&self) -> String {
        format!("{:?}:{}", self.addon_type, self.addon_id)
    }
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
pub enum AddonType {
    Curse,
    Tukui,
    TSM,
}
