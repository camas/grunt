use reqwest::blocking::Client;
use reqwest::header::{HeaderMap, HeaderValue};
use serde::{de::DeserializeOwned, Deserialize, Serialize};

pub const WOW_GAME_ID: i32 = 1;

pub struct CurseAPI {
    client: Client,
}

impl CurseAPI {
    /// Initializes the API
    pub fn init() -> Self {
        let mut headers = HeaderMap::new();
        headers.insert("Accept", HeaderValue::from_static("application/json"));
        headers.insert("Accept-Encoding", HeaderValue::from_static("gzip"));
        let client = Client::builder()
            .default_headers(headers)
            .build()
            .expect("Error creating HTTP client");
        CurseAPI { client }
    }

    pub fn get_game_info(&self, game_id: i32) -> GameInfo {
        self.make_request::<_, (), GameInfo>(format!("game/{}", game_id), None)
    }

    pub fn fingerprint_search(&self, fingerprints: &[u32]) -> FingerprintInfo {
        let info = self.make_request::<_, _, FingerprintInfo>("fingerprint", Some(fingerprints));
        assert!(info
            .partial_match_fingerprints
            .as_object()
            .unwrap()
            .is_empty()); // Never seen and assumed later to be empty. Check to make sure
        info
    }

    fn make_request<S, P, Q>(&self, endpoint: S, data: Option<P>) -> Q
    where
        S: AsRef<str>,
        P: Serialize,
        Q: DeserializeOwned,
    {
        let url = format!(
            "https://addons-ecs.forgesvc.net/api/v2/{}",
            endpoint.as_ref()
        );

        let resp = match data {
            Some(data) => self.client.post(&url).json(&data).send(),
            None => self.client.get(&url).send(),
        }
        .expect("Error making curse api request");
        let resp = resp
            .error_for_status()
            .expect("Error sending curse api request");
        resp.json().expect("Error decoding curse api response")
    }
}

//
// Auto-Generated data classes
//
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GameInfo {
    pub id: i64,
    pub name: String,
    pub slug: String,
    pub date_modified: String,
    pub game_files: Vec<GameFile>,
    pub game_detection_hints: Vec<GameDetectionHint>,
    pub file_parsing_rules: Vec<FileParsingRule>,
    pub category_sections: Vec<CategorySection>,
    pub max_free_storage: i64,
    pub max_premium_storage: i64,
    pub max_file_size: i64,
    pub addon_settings_folder_filter: String,
    pub addon_settings_starting_folder: String,
    pub addon_settings_file_filter: String,
    pub addon_settings_file_removal_filter: String,
    pub supports_addons: bool,
    pub supports_partner_addons: bool,
    pub supported_client_configuration: i64,
    pub supports_notifications: bool,
    pub profiler_addon_id: i64,
    pub twitch_game_id: i64,
    pub client_game_settings_id: i64,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GameFile {
    pub id: i64,
    pub game_id: i64,
    pub is_required: bool,
    pub file_name: String,
    pub file_type: i64,
    pub platform_type: i64,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GameDetectionHint {
    pub id: i64,
    pub hint_type: i64,
    pub hint_path: String,
    pub hint_key: Option<String>,
    pub hint_options: i64,
    pub game_id: i64,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileParsingRule {
    pub comment_strip_pattern: String,
    pub file_extension: String,
    pub inclusion_pattern: String,
    pub game_id: i64,
    pub id: i64,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CategorySection {
    pub id: i64,
    pub game_id: i64,
    pub name: String,
    pub package_type: i64,
    pub path: String,
    pub initial_inclusion_pattern: String,
    pub extra_include_pattern: String,
    pub game_category_id: i64,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FingerprintInfo {
    pub is_cache_built: bool,
    pub exact_matches: Vec<AddonInfo>,
    pub exact_fingerprints: Vec<u32>,
    pub partial_matches: Vec<::serde_json::Value>,
    pub partial_match_fingerprints: ::serde_json::Value,
    pub installed_fingerprints: Vec<u32>,
    pub unmatched_fingerprints: Vec<u32>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AddonInfo {
    pub id: i64,
    pub file: File,
    pub latest_files: Vec<File>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct File {
    pub id: i64,
    pub display_name: String,
    pub file_name: String,
    pub file_date: String,
    pub file_length: i64,
    pub release_type: i64,
    pub file_status: i64,
    pub download_url: String,
    pub is_alternate: bool,
    pub alternate_file_id: i64,
    pub dependencies: Vec<Dependency>,
    pub is_available: bool,
    pub modules: Vec<Module>,
    pub package_fingerprint: u32,
    pub game_version: Vec<String>,
    pub sortable_game_version: Vec<SortableGameVersion>,
    pub install_metadata: ::serde_json::Value,
    pub changelog: ::serde_json::Value,
    pub has_install_script: bool,
    pub is_compatible_with_client: bool,
    pub category_section_package_type: i64,
    pub restrict_project_file_access: i64,
    pub project_status: i64,
    pub render_cache_id: i64,
    pub file_legacy_mapping_id: ::serde_json::Value,
    pub project_id: i64,
    pub parent_project_file_id: ::serde_json::Value,
    pub parent_file_legacy_mapping_id: ::serde_json::Value,
    pub file_type_id: ::serde_json::Value,
    pub expose_as_alternative: ::serde_json::Value,
    pub package_fingerprint_id: i64,
    pub game_version_date_released: String,
    pub game_version_mapping_id: i64,
    pub game_version_id: i64,
    pub game_id: i64,
    pub is_server_pack: bool,
    pub server_pack_file_id: ::serde_json::Value,
    pub game_version_flavor: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Dependency {
    pub id: i64,
    pub addon_id: i64,
    #[serde(rename = "type")]
    pub type_field: i64,
    pub file_id: i64,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Module {
    pub foldername: String,
    pub fingerprint: u32,
    #[serde(rename = "type")]
    pub type_field: i64,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SortableGameVersion {
    pub game_version_padded: String,
    pub game_version: String,
    pub game_version_release_date: String,
    pub game_version_name: String,
}
