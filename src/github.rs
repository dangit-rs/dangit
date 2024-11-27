use anyhow::Context as _;
use reqwest::header::{HeaderMap, HeaderValue};
use secrecy::{ExposeSecret as _, SecretString};

// #[derive(Debug, serde::Deserialize)]
// #[serde(rename_all = "snake_case")]
// enum IssueState {
//     Open,
//     Closed,
// }

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Issue {
    pub html_url: String,
    //     number: u64,
    //     state: IssueState,
    pub title: String,
    pub repository: String,
    issue_type: IssueType,
}

impl std::fmt::Display for Issue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {} → {}", self.repository, self.title, self.html_url)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum IssueType {
    Issue,
    PullRequest,
}

impl std::fmt::Display for IssueType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IssueType::Issue => write!(f, "Issue"),
            IssueType::PullRequest => write!(f, "PR"),
        }
    }
}

#[derive(Debug, serde::Deserialize)]
pub struct Notification {
    // pub reason: String,
    pub subject: Subject,
    // pub unread: bool,
    // pub updated_at: Option<String>,
    // pub last_read_at: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
pub struct Subject {
    pub title: String,
    pub url: String,
    pub latest_comment_url: Option<String>,
}

impl Notification {
    pub fn html_url(&self) -> String {
        // Transforms links of PRs and issues as the following:
        // * https://api.github.com/repos/release-plz/release-plz/issues/1852 -> https://github.com/release-plz/release-plz/issues/1852
        // * https://api.github.com/repos/rust-lang/rust/pulls/132721 -> https://github.com/rust-lang/rust/pull/132721
        self.subject
            .url
            .replace("api.github.com/repos/", "github.com/")
            .replace("/pulls/", "/pull/")
    }
}

impl std::fmt::Display for Notification {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} → {}", self.subject.title, self.html_url())
    }
}

#[derive(Debug, serde::Deserialize)]
struct IssueNode {
    author: Author,
    repository: Repository,
    title: String,
    url: String,
}

#[derive(Debug, serde::Deserialize)]
struct Author {
    login: String,
}

#[derive(Debug, serde::Deserialize)]
struct Repository {
    name: String,
}

pub struct GitHub {
    client: reqwest::Client,
    organization: Option<String>,
}

impl GitHub {
    pub fn new(token: &SecretString, organization: Option<String>) -> anyhow::Result<Self> {
        let client = Self {
            client: client(token)?,
            organization,
        };
        Ok(client)
    }

    pub async fn assigned_issues(&self) -> anyhow::Result<Vec<Issue>> {
        self.get_issues("assignee").await
    }

    pub async fn created_issues(&self) -> anyhow::Result<Vec<Issue>> {
        self.get_issues("author").await
    }

    pub async fn get_issues(&self, filter: &str) -> anyhow::Result<Vec<Issue>> {
        let search_query = match &self.organization {
            Some(org) => format!("org:{org} state:open is:issue {filter}:@me"),
            None => "state:open is:issue {filter}:@me".to_string(),
        };
        // Convert filter string to proper query
        let query = format!(
            r#"
query Issues{{
  search(first: 100, type: ISSUE, query: "{search_query}") {{
    issueCount
    pageInfo {{
      hasNextPage
      endCursor
    }}
    edges {{
      node {{
        ... on Issue {{
          createdAt
          title
          author {{
            login
          }}
          url,
          repository {{
            name
          }}
        }}
      }}
    }}
    }}
}}"#
        );

        let response = self
            .client
            .post("https://api.github.com/graphql")
            .json(&serde_json::json!({
                "query": query,
                // Remove variables since we're using static queries
            }))
            .send()
            .await?
            .json::<serde_json::Value>()
            .await?;

        let nodes: Vec<IssueNode> = response["data"]["search"]["edges"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|n| serde_json::from_value(n["node"].clone()).ok())
            .collect();

        let issues = nodes
            .iter()
            .map(|issue| Issue {
                html_url: issue.url.clone(),
                title: issue.title.clone(),
                repository: issue.repository.name.clone(),
                issue_type: IssueType::Issue,
            })
            .collect();

        Ok(issues)
    }

    pub async fn assigned_prs(&self) -> anyhow::Result<Vec<Issue>> {
        self.get_prs("assignee").await
    }

    pub async fn created_prs(&self) -> anyhow::Result<Vec<Issue>> {
        self.get_prs("author").await
    }

    async fn get_prs(&self, filter: &str) -> anyhow::Result<Vec<Issue>> {
        let search_query = match &self.organization {
            Some(org) => format!("org:{org} state:open is:pr {filter}:@me"),
            None => "state:open is:pr {filter}:@me".to_string(),
        };
        let query = format!(
            r#"
query PullRequests {{
  search(first: 100, type: ISSUE, query: "{search_query}") {{
    issueCount
    edges {{
      node {{
        ... on PullRequest {{
          title
          url
          repository {{
            name
          }}
          author {{
            login
          }}
        }}
      }}
    }}
  }}
}}"#
        );

        let response = self
            .client
            .post("https://api.github.com/graphql")
            .json(&serde_json::json!({
                "query": query,
            }))
            .send()
            .await?
            .json::<serde_json::Value>()
            .await?;

        let nodes: Vec<IssueNode> = response["data"]["search"]["edges"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|n| serde_json::from_value(n["node"].clone()).ok())
            .collect();

        let prs = nodes
            .iter()
            .map(|pr| Issue {
                html_url: pr.url.clone(),
                title: pr.title.clone(),
                repository: pr.repository.name.clone(),
                issue_type: IssueType::PullRequest,
            })
            .collect();

        Ok(prs)
    }

    pub async fn get_notifications(&self) -> anyhow::Result<Vec<Notification>> {
        let response: Vec<Notification> = self
            .client
            .get("https://api.github.com/notifications")
            // with all=true, we get all notifications, including read ones
            // without it, we only get unread notifications.
            // It would be cool to have only "not done" notifications,
            // but it's not possible with the GitHub API.
            // See https://github.com/orgs/community/discussions/118736
            .query(&[("per_page", "50"), ("all", "false")])
            .send()
            .await?
            .json()
            .await?;

        Ok(response)
    }
}

fn default_headers(token: &SecretString) -> anyhow::Result<HeaderMap> {
    let mut headers = HeaderMap::new();
    headers.insert(
        reqwest::header::ACCEPT,
        HeaderValue::from_static("application/vnd.github+json"),
    );
    let mut auth_header: HeaderValue = format!("Bearer {}", token.expose_secret())
        .parse()
        .context("invalid GitHub token")?;
    auth_header.set_sensitive(true);
    headers.insert(reqwest::header::AUTHORIZATION, auth_header);
    Ok(headers)
}

fn client(token: &SecretString) -> anyhow::Result<reqwest::Client> {
    let headers = default_headers(token)?;
    let reqwest_client = reqwest::Client::builder()
        .user_agent("todo-app")
        .default_headers(headers)
        .build()
        .context("can't build Git client")?;
    Ok(reqwest_client)
}
