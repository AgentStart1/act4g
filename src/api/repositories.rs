use super::*;

#[derive(Debug, Clone, Deserialize)]
pub struct Repository {
    pub full_name: String,
    pub private: bool,
    pub has_issues: bool,
}

#[derive(Debug, Clone)]
pub struct RepositorySummary {
    pub repository: Repository,
    pub issues: u64,
    pub pull_requests: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct OpenItem {
    pub number: u64,
    pub title: String,
    pub html_url: String,
    pub user: Option<ItemAuthor>,
    #[serde(default)]
    pub pull_request: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ItemAuthor {
    pub login: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ItemFilter {
    #[default]
    Issues,
    PullRequests,
}

async fn pages<T: serde::de::DeserializeOwned>(
    client: &reqwest::Client,
    token: &str,
    path: &str,
    query: &[(&str, &str)],
) -> anyhow::Result<Vec<T>> {
    let mut items = Vec::new();
    let mut page = 1_u32;
    loop {
        let response = client
            .get(format!("{API_BASE}{path}"))
            .bearer_auth(token)
            .header("User-Agent", "gact/0.1")
            .header("Accept", "application/vnd.github+json")
            .header("X-GitHub-Api-Version", "2022-11-28")
            .query(query)
            .query(&[("per_page", "100"), ("page", &page.to_string())])
            .send()
            .await?
            .error_for_status()?;
        let next = response
            .headers()
            .get(reqwest::header::LINK)
            .and_then(|v| v.to_str().ok())
            .is_some_and(link_has_next_page);
        items.extend(serde_json::from_str::<Vec<T>>(&response.text().await?)?);
        if !next {
            return Ok(items);
        }
        page += 1;
    }
}

// Count connections instead of downloading every issue and PR for the overview.
fn count_query(repositories: &[Repository]) -> anyhow::Result<String> {
    let mut query = String::from("query {");
    for (index, repository) in repositories.iter().enumerate() {
        let (owner, name) = repository
            .full_name
            .split_once('/')
            .ok_or_else(|| anyhow::anyhow!("Invalid repository name"))?;
        query.push_str(&format!(
            "r{index}: repository(owner: {}, name: {}) {{ issues(states: [OPEN]) {{ totalCount }} pullRequests(states: [OPEN]) {{ totalCount }} }}",
            serde_json::to_string(owner)?, serde_json::to_string(name)?
        ));
    }
    query.push('}');
    Ok(query)
}

#[derive(Deserialize)]
struct Count {
    #[serde(rename = "totalCount")]
    total: u64,
}
#[derive(Deserialize)]
struct Counts {
    issues: Count,
    #[serde(rename = "pullRequests")]
    pulls: Count,
}
#[derive(Deserialize)]
struct CountResponse {
    data: Option<std::collections::HashMap<String, Option<Counts>>>,
    #[serde(default)]
    errors: Vec<serde_json::Value>,
}

fn summaries(repositories: &[Repository], text: &str) -> anyhow::Result<Vec<RepositorySummary>> {
    let response: CountResponse = serde_json::from_str(text)?;
    // Never interpret inaccessible repositories or incomplete responses as zero.
    anyhow::ensure!(
        response.errors.is_empty(),
        "GitHub could not count all repositories. Check repository access or API limits."
    );
    let data = response
        .data
        .ok_or_else(|| anyhow::anyhow!("Missing repository counts"))?;
    let mut result = Vec::new();
    for (index, repository) in repositories.iter().enumerate() {
        let counts = data
            .get(&format!("r{index}"))
            .and_then(Option::as_ref)
            .ok_or_else(|| anyhow::anyhow!("Repository counts are unavailable"))?;
        if counts.issues.total > 0 || counts.pulls.total > 0 {
            result.push(RepositorySummary {
                repository: repository.clone(),
                issues: counts.issues.total,
                pull_requests: counts.pulls.total,
            });
        }
    }
    Ok(result)
}

pub async fn get_repository_summaries(
    client: &reqwest::Client,
    token: &str,
) -> anyhow::Result<Vec<RepositorySummary>> {
    let client = client.clone();
    let token = token.to_owned();
    run_http(async move {
        let repositories: Vec<Repository> = pages(
            &client,
            &token,
            "/user/repos",
            &[
                ("visibility", "all"),
                ("affiliation", "owner,collaborator,organization_member"),
                ("sort", "full_name"),
                ("direction", "asc"),
            ],
        )
        .await?;
        let mut result = Vec::new();
        for batch in repositories.chunks(20) {
            let response = client
                .post(format!("{API_BASE}/graphql"))
                .bearer_auth(&token)
                .header("User-Agent", "gact/0.1")
                .header("Content-Type", "application/json")
                .body(serde_json::to_string(
                    &serde_json::json!({"query": count_query(batch)?}),
                )?)
                .send()
                .await?
                .error_for_status()?;
            result.extend(summaries(batch, &response.text().await?)?);
        }
        Ok(result)
    })
    .await
}

pub async fn get_repository_items(
    client: &reqwest::Client,
    token: &str,
    repository: &Repository,
) -> anyhow::Result<Vec<OpenItem>> {
    let client = client.clone();
    let token = token.to_owned();
    let repository = repository.clone();
    run_http(async move {
        let query = [
            ("state", "open"),
            ("sort", "updated"),
            ("direction", "desc"),
        ];
        let mut items: Vec<OpenItem> = if repository.has_issues {
            pages(
                &client,
                &token,
                &format!("/repos/{}/issues", repository.full_name),
                &query,
            )
            .await?
        } else {
            Vec::new()
        };
        // The issues endpoint already contains PRs when issues are enabled.
        if !repository.has_issues {
            items = pages(
                &client,
                &token,
                &format!("/repos/{}/pulls", repository.full_name),
                &query,
            )
            .await?;
            for item in &mut items {
                item.pull_request = Some(serde_json::Value::Null);
            }
        }
        Ok(items)
    })
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    fn repositories() -> Vec<Repository> {
        vec![
            Repository {
                full_name: "org/empty".into(),
                private: true,
                has_issues: true,
            },
            Repository {
                full_name: "org/active".into(),
                private: false,
                has_issues: true,
            },
        ]
    }
    #[test]
    fn hides_empty_repositories_and_keeps_separate_counts() {
        let result = summaries(&repositories(), r#"{"data":{"r0":{"issues":{"totalCount":0},"pullRequests":{"totalCount":0}},"r1":{"issues":{"totalCount":2},"pullRequests":{"totalCount":3}}}}"#).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].repository.full_name, "org/active");
        assert_eq!((result[0].issues, result[0].pull_requests), (2, 3));
    }
    #[test]
    fn keeps_repositories_with_only_issues_or_only_prs() {
        let result = summaries(&repositories(), r#"{"data":{"r0":{"issues":{"totalCount":0},"pullRequests":{"totalCount":4}},"r1":{"issues":{"totalCount":5},"pullRequests":{"totalCount":0}}}}"#).unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!((result[0].issues, result[0].pull_requests), (0, 4));
        assert_eq!((result[1].issues, result[1].pull_requests), (5, 0));
    }
    #[test]
    fn incomplete_counts_are_errors_not_empty_repositories() {
        assert!(summaries(
            &repositories(),
            r#"{"data":{"r0":null},"errors":[{"message":"Forbidden"}]}"#
        )
        .is_err());
        assert!(summaries(&repositories(), r#"{"data":{}}"#).is_err());
    }
    #[test]
    fn distinguishes_prs_in_issue_response() {
        let items: Vec<OpenItem> = serde_json::from_str(r#"[{"number":1,"title":"Issue","html_url":"https://github.com/o/r/issues/1","user":{"login":"a"}},{"number":2,"title":"PR","html_url":"https://github.com/o/r/pull/2","user":{"login":"b"},"pull_request":{"url":"https://api.github.com/repos/o/r/pulls/2"}}]"#).unwrap();
        assert!(items[0].pull_request.is_none());
        assert!(items[1].pull_request.is_some());
    }
    #[test]
    fn accepts_issues_and_prs_without_authors() {
        let items: Vec<OpenItem> = serde_json::from_str(r#"[{"number":1,"title":"Issue","html_url":"https://github.com/o/r/issues/1","user":null},{"number":2,"title":"PR","html_url":"https://github.com/o/r/pull/2","user":null,"pull_request":{"url":"https://api.github.com/repos/o/r/pulls/2"}}]"#).unwrap();
        assert_eq!(items.len(), 2);
        assert!(items.iter().all(|item| item.user.is_none()));
        assert!(items[0].pull_request.is_none());
        assert!(items[1].pull_request.is_some());
    }
    #[test]
    fn counts_query_scopes_each_repository_and_only_open_items() {
        let query = count_query(&repositories()).unwrap();
        assert!(query.contains("r1: repository(owner: \"org\", name: \"active\")"));
        assert_eq!(query.matches("states: [OPEN]").count(), 4);
    }
}
