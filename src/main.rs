mod github;
mod tui;

use std::process::Command;

use crossterm::event::{Event, EventStream, KeyCode};
use futures::StreamExt;
use github::GitHub;
use secrecy::SecretString;
use tui::State;

fn get_github_token() -> SecretString {
    let github_token = Command::new("gh")
        .args(["auth", "token"])
        .output()
        .unwrap()
        .stdout;
    let github_token = String::from_utf8(github_token).unwrap();
    let github_token = github_token.trim();
    let github_token: SecretString = github_token.into();
    github_token
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

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let github_token = get_github_token();
    let client = GitHub::new(&github_token, Some(String::from("ratatui")))?;

    let mut state = State::new(&client).await?;

    let mut terminal = ratatui::init();
    let mut events = EventStream::new();

    while state.is_running {
        terminal.draw(|f| state.draw(f))?;

        if let Some(event) = events.next().await {
            let event = event?;
            match event {
                Event::Key(key_event) => match key_event.code {
                    KeyCode::Char('q') => {
                        state.is_running = false;
                    }
                    KeyCode::Down => {
                        state.list_state.select_next();
                    }
                    KeyCode::Up => {
                        state.list_state.select_previous();
                    }
                    KeyCode::Enter => {
                        if let Some(selected) = state.list_state.selected() {
                            let notification = &state.notifications[selected];
                            webbrowser::open(&notification.html_url())?;
                        }
                    }
                    KeyCode::Tab => {
                        state.selected_tab = (state.selected_tab + 1) % 3;
                        state.list_state.select(Some(0));
                    }
                    _ => {}
                },
                _ => {}
            }

            // state.handle_event(event);
            // terminal.draw(&state);
        }
    }

    ratatui::restore();

    // let notifications = client.get_notifications().await?;
    // if !notifications.is_empty() {
    //     println!("🔔 Notifications ({:?}):", notifications.len());
    // }
    // for notification in &notifications {
    //     println!("  {}", notification);
    // }
    //
    // let assigned_issues = client.assigned_issues().await?;
    // println!("⊙ Assigned issues:");
    // for issue in &assigned_issues {
    //     println!("  {}", issue);
    // }
    // let created_issues = client.created_issues().await?;
    // println!("⊙ Created issues:");
    // for issue in &created_issues {
    //     if !assigned_issues.contains(issue) {
    //         println!("  {}", issue);
    //     }
    // }
    //
    // let assigned_prs = client.assigned_prs().await?;
    // println!("↶ Assigned PRs:");
    // for pr in &assigned_prs {
    //     println!("  {}", pr);
    // }
    // let created_prs = client.created_prs().await?;
    // println!("↶ Created PRs:");
    // for pr in &created_prs {
    //     if !assigned_prs.contains(pr) {
    //         println!("  {}", pr);
    //     }
    // }
    //
    // let all_issues_and_prs = {
    //     let mut seen_urls = std::collections::HashSet::new();
    //     created_issues
    //         .iter()
    //         .chain(assigned_issues.iter())
    //         .chain(created_prs.iter())
    //         .chain(assigned_prs.iter())
    //         .filter(|issue| seen_urls.insert(&issue.html_url))
    //         .collect::<Vec<_>>()
    // };
    // println!("------");
    // print_issues_by_repo(&all_issues_and_prs);
    Ok(())
}
