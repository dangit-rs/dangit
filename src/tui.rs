use std::collections::BTreeMap;

use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Style, Stylize},
    symbols,
    widgets::{Block, List, ListItem, ListState, Tabs},
    Frame,
};
use tachyonfx::{fx, Duration as FxDuration, Effect, EffectRenderer, Interpolation, Shader};

use crate::github::{GitHub, Issue, Notification};

pub struct State {
    pub is_running: bool,
    pub list_state: ListState,
    pub selected_tab: usize,
    pub effect: Effect,

    // GitHub data
    pub assigned_issues: Vec<Issue>,
    pub created_issues: Vec<Issue>,
    pub assigned_prs: Vec<Issue>,
    pub created_prs: Vec<Issue>,
    pub notifications: Vec<Notification>,
}

impl State {
    pub async fn new(client: &GitHub) -> anyhow::Result<Self> {
        Ok(Self {
            is_running: true,
            list_state: {
                let mut list_state = ListState::default();
                list_state.select(Some(0));
                list_state
            },
            effect: fx::coalesce((800, Interpolation::SineOut)),
            selected_tab: 0,
            assigned_issues: client.assigned_issues().await?,
            created_issues: client.created_issues().await?,
            assigned_prs: client.assigned_prs().await?,
            created_prs: client.created_prs().await?,
            notifications: client.get_notifications().await?,
        })
    }

    pub fn draw(&mut self, frame: &mut Frame) {
        let [tab_area, main_area] =
            Layout::vertical([Constraint::Max(3), Constraint::Fill(1)]).areas(frame.area());

        let tabs = Tabs::new(vec!["Notifications", "Issues", "Pull Requests"])
            .block(Block::bordered().title("dangit!"))
            .style(Style::default().white())
            .highlight_style(Style::default().yellow())
            .select(self.selected_tab)
            .divider(symbols::DOT);

        frame.render_widget(tabs, tab_area);

        match self.selected_tab {
            0 => self.draw_notifications(frame, main_area),
            1 => self.draw_issues(frame, main_area, self.assigned_issues.clone()),
            2 => self.draw_issues(frame, main_area, self.created_prs.clone()),
            _ => {}
        }

        if self.effect.running() {
            frame.render_effect(&mut self.effect, frame.area(), FxDuration::from_millis(100));
        }
    }
    fn get_issues_by_repo<'a>(issues: &'a [&'a Issue]) -> BTreeMap<&'a str, Vec<&'a Issue>> {
        let mut issues_by_repo = std::collections::BTreeMap::new();
        for issue in issues {
            let repo = issue.repository.as_str();
            let entry = issues_by_repo.entry(repo).or_insert_with(Vec::new);
            entry.push(*issue);
        }
        issues_by_repo
    }

    fn draw_notifications(&mut self, frame: &mut Frame, area: Rect) {
        let items = self
            .notifications
            .iter()
            .map(|notification| notification.subject.title.clone())
            .collect::<Vec<_>>();

        let list = List::new(items)
            .block(Block::bordered().title("List"))
            .style(Style::new().white())
            .highlight_style(Style::default().reversed().italic())
            .highlight_symbol("👉 ")
            .repeat_highlight_symbol(true);

        frame.render_stateful_widget(list, area, &mut self.list_state);
    }

    fn draw_issues(&mut self, frame: &mut Frame, area: Rect, issues: Vec<Issue>) {
        let [repo_list, issue_list] =
            Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)])
                .areas(area);

        let issues = issues.iter().collect::<Vec<_>>();
        let repo_issue_map = Self::get_issues_by_repo(&issues);

        let repos = repo_issue_map
            .keys()
            .into_iter()
            .map(|v| String::from(*v))
            .collect::<Vec<String>>();

        let list = List::new(
            repos
                .clone()
                .into_iter()
                .map(|v| ListItem::new(v))
                .collect::<Vec<_>>(),
        )
        .block(Block::bordered().title("Repositories"))
        .style(Style::new().white())
        .highlight_style(Style::default().reversed().italic())
        .highlight_symbol("👉 ")
        .repeat_highlight_symbol(true);

        frame.render_stateful_widget(list, repo_list, &mut self.list_state);

        let selected_repo = &repos[self.list_state.selected().unwrap()];

        let items = issues
            .iter()
            .filter(|issue| issue.repository == *selected_repo)
            .map(|issue| issue.title.clone())
            .collect::<Vec<_>>();

        let list = List::new(items)
            .block(Block::bordered().title("Issues"))
            .style(Style::new().white())
            .highlight_style(Style::default().reversed().italic())
            .highlight_symbol("👉 ")
            .repeat_highlight_symbol(true);

        frame.render_widget(list, issue_list);
    }
}
