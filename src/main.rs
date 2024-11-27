mod github;

use std::process::Command;

use github::GitHub;
use secrecy::SecretString;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let github_token = Command::new("gh")
        .args(["auth", "token"])
        .output()
        .unwrap()
        .stdout;
    let github_token = String::from_utf8(github_token).unwrap();
    let github_token = github_token.trim();
    let github_token: SecretString = github_token.into();

    let client = GitHub::new(&github_token)?;

    let notifications = client.get_notifications().await?;
    if !notifications.is_empty() {
        println!("🔔 Notifications ({:?}):", notifications.len());
    }
    for notification in &notifications {
        println!("  {}", notification);
    }

    let assigned_issues = client.assigned_issues().await?;
    println!("⊙ Assigned issues:");
    for issue in &assigned_issues {
        println!("  {}", issue);
    }
    let created_issues = client.created_issues().await?;
    println!("⊙ Created issues:");
    for issue in &created_issues {
        if !assigned_issues.contains(issue) {
            println!("  {}", issue);
        }
    }

    let assigned_prs = client.assigned_prs().await?;
    println!("↶ Assigned PRs:");
    for pr in &assigned_prs {
        println!("  {}", pr);
    }
    let created_prs = client.created_prs().await?;
    println!("↶ Created PRs:");
    for pr in &created_prs {
        if !assigned_prs.contains(pr) {
            println!("  {}", pr);
        }
    }

    let all_issues_and_prs = {
        let mut seen_urls = std::collections::HashSet::new();
        created_issues
            .iter()
            .chain(assigned_issues.iter())
            .chain(created_prs.iter())
            .chain(assigned_prs.iter())
            .filter(|issue| seen_urls.insert(&issue.html_url))
            .collect::<Vec<_>>()
    };
    println!("------");
    print_issues_by_repo(&all_issues_and_prs);
    Ok(())
}

fn print_issues_by_repo(issues: &[&github::Issue]) {
    let mut issues_by_repo = std::collections::BTreeMap::new();
    for issue in issues {
        let repo = &issue.repository;
        let entry = issues_by_repo.entry(repo).or_insert_with(Vec::new);
        entry.push(issue);
    }
    for (repo, issues) in issues_by_repo {
        println!("{}:", repo);
        for issue in issues {
            println!("  {}", issue);
        }
    }
}
