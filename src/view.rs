use chrono::{DateTime, Local};

use crate::accounts::AccountRank;
use crate::model::Reply;
use crate::report;

pub trait View {
    fn title(&self) -> &'static str;
    fn render(&self, data: &Dataset, now: DateTime<Local>) -> String;
}

pub struct Dataset {
    pub handle: String,
    pub counts: Vec<u64>,
    pub yesterday: Vec<Reply>,
    pub ranks: Vec<AccountRank>,
    pub today_count: u64,
}

pub fn all() -> Vec<Box<dyn View>> {
    vec![
        Box::new(GoalGrid),
        Box::new(Welcome),
        Box::new(Yesterday),
        Box::new(TopAccounts),
        Box::new(DailyGoal),
    ]
}

struct GoalGrid;

struct Welcome;

struct Yesterday;

struct TopAccounts;

struct DailyGoal;

impl View for GoalGrid {
    fn title(&self) -> &'static str {
        "Last 30 days"
    }

    fn render(&self, data: &Dataset, _now: DateTime<Local>) -> String {
        let grid = report::render_grid(&data.counts);
        grid.split_once('\n')
            .map(|(_, squares)| squares.to_string())
            .unwrap_or_default()
    }
}

impl View for Welcome {
    fn title(&self) -> &'static str {
        "Welcome"
    }

    fn render(&self, data: &Dataset, _now: DateTime<Local>) -> String {
        format!("{} · connected", data.handle)
    }
}

impl View for Yesterday {
    fn title(&self) -> &'static str {
        ""
    }

    fn render(&self, data: &Dataset, now: DateTime<Local>) -> String {
        report::render(&data.yesterday, &now.timezone())
    }
}

impl View for TopAccounts {
    fn title(&self) -> &'static str {
        "Accounts"
    }

    fn render(&self, data: &Dataset, _now: DateTime<Local>) -> String {
        let accounts = report::render_accounts(&data.ranks);
        accounts
            .split_once('\n')
            .map(|(_, rows)| rows.to_string())
            .unwrap_or_default()
    }
}

impl View for DailyGoal {
    fn title(&self) -> &'static str {
        ""
    }

    fn render(&self, data: &Dataset, _now: DateTime<Local>) -> String {
        report::render_today(data.today_count)
    }
}
