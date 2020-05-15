use data_encoding::HEXLOWER;
use reqwest::blocking::{Client, ClientBuilder};
use ring::digest::{Algorithm, Context, SHA256, SHA512};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

const PASSWORD_SALT: &str = "f2f618c502a975825e5da6f8650ba8fb";
const TOKEN_SALT: &str = "6e8fd9d5da4f1cd0e64ad4d082be477c";
pub const APP_VERSION: u32 = 403;

pub struct TSMApi {
    clients: HashMap<String, Client>,
    session: String,
    subdomains: HashMap<String, String>,
}

impl TSMApi {
    pub fn new() -> TSMApi {
        let mut subdomains: HashMap<String, String> = HashMap::new();
        subdomains.insert("login".into(), "app-server".into());
        subdomains.insert("log".into(), "app-server".into());
        TSMApi {
            clients: HashMap::new(),
            session: "".into(),
            subdomains,
        }
    }

    /// Login to the TSM Api
    pub fn login(&mut self, email: &str, password: &str) {
        self.create_clients();
        let email_hash = hash_string(&email.to_ascii_lowercase(), &SHA256);
        let initial_pass_hash = hash_string(password, &SHA512);
        let pass_hash = hash_string(&format!("{}{}", initial_pass_hash, PASSWORD_SALT), &SHA512);
        let user_info = self.make_request::<LoginRespData>(vec!["login", &email_hash, &pass_hash]);
        self.session = user_info.session;
        self.subdomains.extend(user_info.endpoint_subdomains);
        self.create_clients();
    }

    pub fn get_status(&self) -> StatusRespData {
        self.make_request::<StatusRespData>(vec!["status"])
    }

    pub fn auctiondb(&self, data_type: &str, id: i64) -> String {
        let resp =
            self.make_request::<AuctionDBRespData>(vec!["auctiondb", data_type, &id.to_string()]);
        resp.data
    }

    /// Downloads a TSM addon the the specified path
    pub fn addon<P>(&self, addon_name: &str, path: P)
    where
        P: AsRef<Path>,
    {
        let mut resp = self.make_request_raw(vec!["addon", addon_name]);
        let file = std::fs::File::create(path).unwrap();
        let mut writer = std::io::BufWriter::new(file);
        resp.copy_to(&mut writer).unwrap();
    }

    fn create_clients(&mut self) {
        for (_, subdomain) in self.subdomains.iter() {
            self.clients
                .entry(subdomain.into())
                .or_insert_with(|| ClientBuilder::new().build().unwrap());
        }
    }

    fn make_request<T: serde::de::DeserializeOwned>(&self, endpoint: Vec<&str>) -> T {
        let resp = self.make_request_raw(endpoint);
        resp.json::<T>().unwrap()
    }

    fn make_request_raw(&self, endpoint: Vec<&str>) -> reqwest::blocking::Response {
        // Setup params
        let session = &self.session;
        let version = APP_VERSION.to_string();
        let time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
            .to_string();
        let token = hash_string(&format!("{}:{}:{}", APP_VERSION, time, TOKEN_SALT), &SHA256);
        let channel = "release";
        let tsm_version = "";
        let mut params: HashMap<&str, &str> = HashMap::new();
        params.insert("session", session);
        params.insert("version", &version);
        params.insert("time", &time);
        params.insert("token", &token);
        params.insert("channel", channel);
        params.insert("tsm_version", tsm_version);

        // Get subdomain
        let subdomain = self
            .subdomains
            .get(endpoint[0])
            .expect("Subdomain not found for endpoint");

        // Get client
        let client = self
            .clients
            .get(subdomain)
            .expect("Client not found for subdomain");

        // Make request
        let url = format!(
            "http://{}.tradeskillmaster.com/v2/{}",
            subdomain,
            endpoint.join("/")
        );
        client.get(&url).query(&params).send().unwrap()
    }
}

fn hash_string(data: &str, algorithm: &'static Algorithm) -> String {
    let mut context = Context::new(algorithm);
    let bytes = data.as_bytes();
    context.update(&bytes);
    let digest = context.finish();
    HEXLOWER.encode(digest.as_ref())
}

#[derive(Default, Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct AuctionDBRespData {
    pub success: bool,
    pub data: String,
}

#[derive(Default, Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct LoginRespData {
    pub session: String,
    pub endpoint_subdomains: HashMap<String, String>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StatusRespData {
    pub success: bool,
    pub name: String,
    pub app_info: AppInfo,
    pub addon_message: AddonMessage,
    pub addon_news: String,
    pub realms: Vec<Realm>,
    #[serde(rename = "realms-Classic")]
    pub realms_classic: Vec<::serde_json::Value>,
    pub regions: Vec<Region>,
    #[serde(rename = "regions-Classic")]
    pub regions_classic: Vec<::serde_json::Value>,
    pub addons: Vec<Addon>,
    #[serde(rename = "addons-Classic")]
    pub addons_classic: Vec<AddonsClassic>,
    pub channels: Channels,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    pub news: String,
    #[serde(rename = "minTSMUpdateNotificationVersion")]
    pub min_tsmupdate_notification_version: i64,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AddonMessage {
    pub id: i64,
    pub msg: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Realm {
    pub id: i64,
    pub master_id: i64,
    pub name: String,
    pub region: String,
    pub last_modified: u64,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Region {
    pub id: i64,
    pub name: String,
    pub last_modified: u64,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Addon {
    pub name: String,
    #[serde(rename = "version_str")]
    pub version_str: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AddonsClassic {
    pub name: String,
    #[serde(rename = "version_str")]
    pub version_str: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Channels {
    pub release: String,
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_login() {
        dotenv::dotenv().ok();
        let email = env::var("TSM_TEST_EMAIL").unwrap();
        let password = env::var("TSM_TEST_PASSWORD").unwrap();
        let mut api = TSMApi::new();
        api.login(&email, &password);
    }
}
