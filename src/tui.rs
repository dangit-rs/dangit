use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Style, Stylize},
    symbols,
    widgets::{Block, List, ListState, Tabs},
    Frame,
};

use crate::github::{GitHub, Issue, Notification};

pub struct State {
    pub is_running: bool,
    pub list_state: ListState,
    pub selected_tab: usize,

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
        let items = issues
            .iter()
            .map(|issue| issue.title.clone())
            .collect::<Vec<_>>();

        let list = List::new(items)
            .block(Block::bordered().title("Issues"))
            .style(Style::new().white())
            .highlight_style(Style::default().reversed().italic())
            .highlight_symbol("👉 ")
            .repeat_highlight_symbol(true);

        frame.render_stateful_widget(list, area, &mut self.list_state);
    }
}
