use serde::Deserialize;

pub const CLIENT_ID: &str = env!("CLIENT_ID");

const DEVICE_CODE_URL: &str = "https://github.com/login/device/code";
const TOKEN_URL: &str = "https://github.com/login/oauth/access_token";
const API_BASE: &str = "https://api.github.com";

#[derive(Deserialize, Debug, Clone)]
pub struct DeviceCodeResponse {
    pub device_code: String,
    pub user_code: String,
    pub verification_uri: String,
    pub expires_in: u64,
    pub interval: u64,
}

#[derive(Deserialize, Debug)]
pub struct TokenResponse {
    pub access_token: Option<String>,
    pub error: Option<String>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct GitHubUser {
    pub login: String,
    pub name: Option<String>,
    pub avatar_url: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Notification {
    pub id: String,
    pub repository: NotifRepo,
    pub subject: NotifSubject,
    pub reason: String,
    pub updated_at: String,
    pub unread: bool,
}

#[derive(Deserialize, Debug, Clone)]
pub struct NotifRepo {
    pub full_name: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct NotifSubject {
    pub title: String,
    #[serde(rename = "type")]
    pub kind: String,
}

pub fn make_client() -> anyhow::Result<reqwest::Client> {
    Ok(reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()?)
}

pub async fn request_device_code(client: &reqwest::Client) -> anyhow::Result<DeviceCodeResponse> {
    let text = client
        .post(DEVICE_CODE_URL)
        .header("Accept", "application/json")
        .form(&[("client_id", CLIENT_ID), ("scope", "notifications read:user")])
        .send()
        .await?
        .text()
        .await?;
    serde_json::from_str(&text).map_err(|e| anyhow::anyhow!("parse device code: {e}\nbody: {text}"))
}

pub async fn poll_token(client: &reqwest::Client, device_code: &str) -> anyhow::Result<TokenResponse> {
    let text = client
        .post(TOKEN_URL)
        .header("Accept", "application/json")
        .form(&[
            ("client_id", CLIENT_ID),
            ("device_code", device_code),
            ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
        ])
        .send()
        .await?
        .text()
        .await?;
    serde_json::from_str(&text).map_err(|e| anyhow::anyhow!("parse token: {e}\nbody: {text}"))
}

pub async fn get_user(client: &reqwest::Client, token: &str) -> anyhow::Result<GitHubUser> {
    let text = client
        .get(format!("{API_BASE}/user"))
        .header("Authorization", format!("Bearer {token}"))
        .header("User-Agent", "act4g/0.1")
        .header("Accept", "application/vnd.github+json")
        .header("X-GitHub-Api-Version", "2022-11-28")
        .send()
        .await?
        .text()
        .await?;
    serde_json::from_str(&text).map_err(|e| anyhow::anyhow!("parse user: {e}\nbody: {text}"))
}

pub async fn get_notifications(client: &reqwest::Client, token: &str) -> anyhow::Result<Vec<Notification>> {
    let text = client
        .get(format!("{API_BASE}/notifications"))
        .header("Authorization", format!("Bearer {token}"))
        .header("User-Agent", "act4g/0.1")
        .header("Accept", "application/vnd.github+json")
        .header("X-GitHub-Api-Version", "2022-11-28")
        .send()
        .await?
        .text()
        .await?;
    serde_json::from_str(&text).map_err(|e| anyhow::anyhow!("parse notifications: {e}\nbody: {text}"))
}
