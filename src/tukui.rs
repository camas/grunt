use serde::{de::DeserializeOwned, Deserialize, Serialize};

pub fn get_addon_infos() -> Vec<AddonInfo> {
    make_request("client-api.php?addons=all")
}

pub fn get_elvui_info() -> ElvUIInfo {
    make_request("client-api.php?ui=elvui")
}

/// Makes a request to a Tukui API endpoint, decoding the response as json
fn make_request<Q>(endpoint: &str) -> Q
where
    Q: DeserializeOwned,
{
    let url = format!("https://www.tukui.org/{}", endpoint);

    let resp = reqwest::blocking::get(&url).expect("Error making tukui api request");
    let resp = resp
        .error_for_status()
        .expect("Error sending tukui api request");
    resp.json().expect("Error decoding curse api response")
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AddonInfo {
    pub id: String,
    pub name: String,
    #[serde(rename = "small_desc")]
    pub small_desc: String,
    pub author: String,
    pub version: String,
    #[serde(rename = "screenshot_url")]
    pub screenshot_url: String,
    pub url: String,
    pub category: Option<String>,
    pub downloads: String,
    pub lastupdate: String,
    pub patch: String,
    #[serde(rename = "web_url")]
    pub web_url: String,
    #[serde(rename = "last_download")]
    pub last_download: String,
    pub changelog: Option<String>,
    #[serde(rename = "donate_url")]
    pub donate_url: Option<String>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ElvUIInfo {
    pub name: String,
    pub author: String,
    pub url: String,
    pub version: String,
    pub changelog: String,
    pub ticket: String,
    pub git: String,
    pub id: i64,
    pub patch: String,
    pub lastupdate: String,
    #[serde(rename = "web_url")]
    pub web_url: String,
    pub lastdownload: String,
    #[serde(rename = "donate_url")]
    pub donate_url: String,
    #[serde(rename = "small_desc")]
    pub small_desc: String,
    #[serde(rename = "screenshot_url")]
    pub screenshot_url: String,
    pub downloads: i64,
    pub category: String,
}
