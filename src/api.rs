use serde::Deserialize;
use std::future::Future;

pub const CLIENT_ID: &str = match option_env!("CLIENT_ID") {
    Some(client_id) => client_id,
    None => "",
};

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
    if CLIENT_ID.is_empty() {
        anyhow::bail!("GitHub OAuth client ID is not configured");
    }

    let client = client.clone();

    run_http(async move {
        let text = client
            .post(DEVICE_CODE_URL)
            .header("Accept", "application/json")
            .form(&[
                ("client_id", CLIENT_ID),
                ("scope", "notifications read:user repo"),
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
        let mut notifications = Vec::new();
        let mut page = 1_u32;

        loop {
            let response = client
                .get(format!("{API_BASE}/notifications"))
                .query(&[
                    ("all", "true"),
                    ("per_page", "50"),
                    ("page", &page.to_string()),
                ])
                .header("Authorization", format!("Bearer {token}"))
                .header("User-Agent", "act4g/0.1")
                .header("Accept", "application/vnd.github+json")
                .header("X-GitHub-Api-Version", "2022-11-28")
                .send()
                .await?
                .error_for_status()?;

            let has_next_page = response
                .headers()
                .get(reqwest::header::LINK)
                .and_then(|value| value.to_str().ok())
                .is_some_and(link_has_next_page);
            let text = response.text().await?;
            let mut page_notifications: Vec<Notification> = serde_json::from_str(&text)
                .map_err(|e| anyhow::anyhow!("parse notifications: {e}\nbody: {text}"))?;
            notifications.append(&mut page_notifications);

            if !has_next_page {
                break;
            }
            page += 1;
        }

        Ok(notifications)
    })
    .await
}

pub async fn mark_notification_as_read(
    client: &reqwest::Client,
    token: &str,
    notification_id: &str,
) -> anyhow::Result<()> {
    let client = client.clone();
    let token = token.to_owned();
    let notification_id = notification_id.to_owned();

    run_http(async move {
        client
            .patch(format!(
                "{API_BASE}/notifications/threads/{notification_id}"
            ))
            .header("Authorization", format!("Bearer {token}"))
            .header("User-Agent", "act4g/0.1")
            .header("Accept", "application/vnd.github+json")
            .header("X-GitHub-Api-Version", "2022-11-28")
            .send()
            .await?
            .error_for_status()?;
        Ok(())
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
            .get(&subject_url)
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

        let html_url = string("html_url").or_else(|| {
            value
                .get("head_sha")
                .and_then(serde_json::Value::as_str)
                .and_then(|head_sha| check_suite_web_url(&subject_url, head_sha))
        });

        Ok(NotificationDetail {
            html_url,
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

    let path = match notification.subject.kind.as_str() {
        "PullRequest" => path.replace("/pulls/", "/pull/"),
        "Commit" => path.replace("/commits/", "/commit/"),
        "Issue" | "Discussion" => path.to_owned(),
        _ => return notification.repository.html_url.clone(),
    };
    format!("https://github.com/{path}")
}

pub fn notification_detail_url(notification: &Notification) -> Option<&str> {
    notification
        .subject
        .latest_comment_url
        .as_deref()
        .or(notification.subject.url.as_deref())
}

fn link_has_next_page(link: &str) -> bool {
    link.split(',').any(|part| part.contains("rel=\"next\""))
}

fn check_suite_web_url(api_url: &str, head_sha: &str) -> Option<String> {
    let path = api_url.strip_prefix("https://api.github.com/repos/")?;
    let repository = path.split_once("/check-suites/")?.0;
    Some(format!(
        "https://github.com/{repository}/commit/{head_sha}/checks"
    ))
}

pub fn is_unauthorized(error: &anyhow::Error) -> bool {
    error
        .downcast_ref::<reqwest::Error>()
        .and_then(reqwest::Error::status)
        == Some(reqwest::StatusCode::UNAUTHORIZED)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn notification(kind: &str, url: &str, latest_comment_url: Option<&str>) -> Notification {
        Notification {
            id: "1".to_string(),
            repository: NotifRepo {
                full_name: "octocat/hello-world".to_string(),
                html_url: "https://github.com/octocat/hello-world".to_string(),
            },
            subject: NotifSubject {
                title: "Hello".to_string(),
                kind: kind.to_string(),
                url: Some(url.to_string()),
                latest_comment_url: latest_comment_url.map(str::to_owned),
            },
            reason: "comment".to_string(),
            updated_at: "2026-09-02T00:00:00Z".to_string(),
            unread: true,
        }
    }

    #[test]
    fn detects_next_page_in_link_header() {
        assert!(link_has_next_page(
            "<https://api.github.com/notifications?page=2>; rel=\"next\", <https://api.github.com/notifications?page=4>; rel=\"last\""
        ));
        assert!(!link_has_next_page(
            "<https://api.github.com/notifications?page=1>; rel=\"prev\""
        ));
    }

    #[test]
    fn prefers_latest_comment_for_notification_detail() {
        let notification = notification(
            "PullRequest",
            "https://api.github.com/repos/octocat/hello-world/pulls/7",
            Some("https://api.github.com/repos/octocat/hello-world/issues/comments/9"),
        );

        assert_eq!(
            notification_detail_url(&notification),
            Some("https://api.github.com/repos/octocat/hello-world/issues/comments/9")
        );
    }

    #[test]
    fn converts_supported_subject_urls_to_web_urls() {
        let pull_request = notification(
            "PullRequest",
            "https://api.github.com/repos/octocat/hello-world/pulls/7",
            None,
        );
        let check_suite = notification(
            "CheckSuite",
            "https://api.github.com/repos/octocat/hello-world/check-suites/5",
            None,
        );

        assert_eq!(
            notification_web_url(&pull_request),
            "https://github.com/octocat/hello-world/pull/7"
        );
        assert_eq!(
            notification_web_url(&check_suite),
            "https://github.com/octocat/hello-world"
        );
        assert_eq!(
            check_suite_web_url(
                "https://api.github.com/repos/octocat/hello-world/check-suites/5",
                "abc123"
            ),
            Some("https://github.com/octocat/hello-world/commit/abc123/checks".to_string())
        );
    }
}
