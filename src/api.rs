use serde::Deserialize;
use std::future::Future;

pub const CLIENT_ID: &str = env!("CLIENT_ID");

const DEVICE_CODE_URL: &str = "https://github.com/login/device/code";
const TOKEN_URL: &str = "https://github.com/login/oauth/access_token";
const API_BASE: &str = "https://api.github.com";

#[cfg(target_os = "android")]
fn android_system_proxy_url() -> anyhow::Result<Option<String>> {
    use jni::objects::{JObject, JString};

    let vm = jni::JavaVM::singleton()?;
    vm.attach_current_thread(|env| -> anyhow::Result<Option<String>> {
        let host_object: JObject = env
            .call_static_method(
                jni::jni_str!("android/net/Proxy"),
                jni::jni_str!("getDefaultHost"),
                jni::jni_sig!("()Ljava/lang/String;"),
                &[],
            )?
            .l()?;

        if host_object.is_null() {
            return Ok(None);
        }

        let host = JString::cast_local(env, host_object)?.try_to_string(env)?;
        let port = env
            .call_static_method(
                jni::jni_str!("android/net/Proxy"),
                jni::jni_str!("getDefaultPort"),
                jni::jni_sig!("()I"),
                &[],
            )?
            .i()?;

        if host.is_empty() || !(1..=u16::MAX as i32).contains(&port) {
            return Ok(None);
        }

        let host = if host.contains(':') && !host.starts_with('[') {
            format!("[{host}]")
        } else {
            host
        };

        Ok(Some(format!("http://{host}:{port}")))
    })
}

fn proxy_url() -> Option<String> {
    #[cfg(target_os = "android")]
    match android_system_proxy_url() {
        Ok(Some(proxy_url)) => {
            log::info!("Using Android system proxy {proxy_url}");
            return Some(proxy_url);
        }
        Ok(None) => {}
        Err(error) => log::warn!("Failed to read Android system proxy: {error:#}"),
    }

    option_env!("ACT4G_HTTP_PROXY").map(str::to_owned)
}

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
    pub html_url: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct NotifSubject {
    pub title: String,
    #[serde(rename = "type")]
    pub kind: String,
    pub url: Option<String>,
    pub latest_comment_url: Option<String>,
}

#[derive(Debug, Clone)]
pub struct NotificationDetail {
    pub html_url: Option<String>,
    pub body: Option<String>,
    pub state: Option<String>,
    pub author: Option<String>,
    pub comments: Option<u64>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

pub fn make_client() -> anyhow::Result<reqwest::Client> {
    let _runtime_guard = reqwest_client::runtime().enter();

    let mut builder = reqwest::Client::builder().timeout(std::time::Duration::from_secs(30));

    if let Some(proxy_url) = proxy_url() {
        builder = builder.proxy(reqwest::Proxy::all(&proxy_url)?);
    }

    Ok(builder.build()?)
}

async fn run_http<F, T>(future: F) -> anyhow::Result<T>
where
    F: Future<Output = anyhow::Result<T>> + Send + 'static,
    T: Send + 'static,
{
    reqwest_client::runtime()
        .spawn(future)
        .await
        .map_err(|error| anyhow::anyhow!("HTTP runtime task failed: {error}"))?
}

pub async fn request_device_code(client: &reqwest::Client) -> anyhow::Result<DeviceCodeResponse> {
    let client = client.clone();

    run_http(async move {
        let text = client
            .post(DEVICE_CODE_URL)
            .header("Accept", "application/json")
            .form(&[
                ("client_id", CLIENT_ID),
                ("scope", "notifications read:user"),
            ])
            .send()
            .await?
            .error_for_status()?
            .text()
            .await?;

        serde_json::from_str(&text)
            .map_err(|e| anyhow::anyhow!("parse device code: {e}\nbody: {text}"))
    })
    .await
}

pub async fn poll_token(
    client: &reqwest::Client,
    device_code: &str,
) -> anyhow::Result<TokenResponse> {
    let client = client.clone();
    let device_code = device_code.to_owned();

    run_http(async move {
        let text = client
            .post(TOKEN_URL)
            .header("Accept", "application/json")
            .form(&[
                ("client_id", CLIENT_ID),
                ("device_code", device_code.as_str()),
                ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
            ])
            .send()
            .await?
            .error_for_status()?
            .text()
            .await?;

        serde_json::from_str(&text).map_err(|e| anyhow::anyhow!("parse token: {e}\nbody: {text}"))
    })
    .await
}

pub async fn get_user(client: &reqwest::Client, token: &str) -> anyhow::Result<GitHubUser> {
    let client = client.clone();
    let token = token.to_owned();

    run_http(async move {
        let text = client
            .get(format!("{API_BASE}/user"))
            .header("Authorization", format!("Bearer {token}"))
            .header("User-Agent", "act4g/0.1")
            .header("Accept", "application/vnd.github+json")
            .header("X-GitHub-Api-Version", "2022-11-28")
            .send()
            .await?
            .error_for_status()?
            .text()
            .await?;

        serde_json::from_str(&text).map_err(|e| anyhow::anyhow!("parse user: {e}\nbody: {text}"))
    })
    .await
}

pub async fn get_notifications(
    client: &reqwest::Client,
    token: &str,
) -> anyhow::Result<Vec<Notification>> {
    let client = client.clone();
    let token = token.to_owned();

    run_http(async move {
        let text = client
            .get(format!("{API_BASE}/notifications"))
            .header("Authorization", format!("Bearer {token}"))
            .header("User-Agent", "act4g/0.1")
            .header("Accept", "application/vnd.github+json")
            .header("X-GitHub-Api-Version", "2022-11-28")
            .send()
            .await?
            .error_for_status()?
            .text()
            .await?;

        serde_json::from_str(&text)
            .map_err(|e| anyhow::anyhow!("parse notifications: {e}\nbody: {text}"))
    })
    .await
}

pub async fn get_notification_detail(
    client: &reqwest::Client,
    token: &str,
    subject_url: &str,
) -> anyhow::Result<NotificationDetail> {
    if !subject_url.starts_with("https://api.github.com/") {
        anyhow::bail!("GitHub returned an unsupported notification URL");
    }

    let client = client.clone();
    let token = token.to_owned();
    let subject_url = subject_url.to_owned();

    run_http(async move {
        let text = client
            .get(subject_url)
            .header("Authorization", format!("Bearer {token}"))
            .header("User-Agent", "act4g/0.1")
            .header("Accept", "application/vnd.github+json")
            .header("X-GitHub-Api-Version", "2022-11-28")
            .send()
            .await?
            .error_for_status()?
            .text()
            .await?;

        let value: serde_json::Value = serde_json::from_str(&text)
            .map_err(|error| anyhow::anyhow!("parse notification detail: {error}"))?;

        let string = |key: &str| {
            value
                .get(key)
                .and_then(serde_json::Value::as_str)
                .map(str::to_owned)
        };

        Ok(NotificationDetail {
            html_url: string("html_url"),
            body: string("body").filter(|body| !body.trim().is_empty()),
            state: string("state").or_else(|| string("status")),
            author: value
                .pointer("/user/login")
                .or_else(|| value.pointer("/author/login"))
                .and_then(serde_json::Value::as_str)
                .map(str::to_owned),
            comments: value.get("comments").and_then(serde_json::Value::as_u64),
            created_at: string("created_at"),
            updated_at: string("updated_at"),
        })
    })
    .await
}

pub fn notification_web_url(notification: &Notification) -> String {
    let Some(api_url) = notification.subject.url.as_deref() else {
        return notification.repository.html_url.clone();
    };

    let Some(path) = api_url.strip_prefix("https://api.github.com/repos/") else {
        return notification.repository.html_url.clone();
    };

    let path = path.replace("/pulls/", "/pull/");
    format!("https://github.com/{path}")
}

pub fn is_unauthorized(error: &anyhow::Error) -> bool {
    error
        .downcast_ref::<reqwest::Error>()
        .and_then(reqwest::Error::status)
        == Some(reqwest::StatusCode::UNAUTHORIZED)
}
