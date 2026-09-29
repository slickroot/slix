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

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};

    fn dataset() -> Dataset {
        Dataset {
            handle: "me".into(),
            counts: Vec::new(),
            yesterday: Vec::new(),
            ranks: Vec::new(),
            today_count: 0,
        }
    }

    fn fixed_now() -> DateTime<Local> {
        Local.with_ymd_and_hms(2026, 3, 11, 12, 0, 0).unwrap()
    }

    fn reply(text: &str) -> Reply {
        Reply {
            id: "123".into(),
            created_at: "2026-03-10T23:30:00Z".parse::<DateTime<Utc>>().unwrap(),
            text: text.into(),
            to_username: "alice".into(),
            impressions: 42,
            likes: 0,
            profile_visits: 0,
        }
    }

    fn reply_at(
        created_at: &str,
        to_username: &str,
        text: &str,
        impressions: u64,
        likes: u64,
        profile_visits: u64,
    ) -> Reply {
        Reply {
            id: "1".into(),
            created_at: created_at.parse::<DateTime<Utc>>().unwrap(),
            text: text.into(),
            to_username: to_username.into(),
            impressions,
            likes,
            profile_visits,
        }
    }

    fn grid_squares(counts: &[u64]) -> String {
        GoalGrid.render(
            &Dataset {
                counts: counts.to_vec(),
                ..dataset()
            },
            fixed_now(),
        )
    }

    fn account(handle: &str, avg_impressions: f64, reply_count: usize) -> AccountRank {
        AccountRank {
            handle: handle.into(),
            avg_impressions,
            reply_count,
        }
    }

    fn strip_osc8(s: &str) -> String {
        let mut out = String::new();
        let mut rest = s;
        while let Some(start) = rest.find("\x1b]8;;") {
            out.push_str(&rest[..start]);
            let after_start = &rest[start..];
            match after_start.find("\x1b\\") {
                Some(end) => rest = &after_start[end + "\x1b\\".len()..],
                None => {
                    rest = "";
                    break;
                }
            }
        }
        out.push_str(rest);
        out
    }

    #[test]
    fn shows_local_time_handle_and_impressions() {
        let fixture = reply("hi");
        let out = Yesterday.render(
            &Dataset {
                yesterday: vec![fixture.clone()],
                ..dataset()
            },
            fixed_now(),
        );
        let time = fixture
            .created_at
            .with_timezone(&Local)
            .format("%H:%M")
            .to_string();
        assert!(out.contains(&time));
        assert!(out.contains("@alice"));
        assert!(out.contains("42 impressions"));
    }

    #[test]
    fn aligns_columns_across_replies() {
        let out = Yesterday.render(
            &Dataset {
                yesterday: vec![
                    reply_at("2026-03-10T10:00:00Z", "al", "short", 5, 5, 5),
                    reply_at(
                        "2026-03-10T09:00:00Z",
                        "bobby",
                        "longer text",
                        1234,
                        1234,
                        1234,
                    ),
                ],
                ..dataset()
            },
            fixed_now(),
        );
        let preview_width = report::PREVIEW_WIDTH;
        let lines: Vec<&str> = out.lines().skip(1).collect();
        assert!(lines[0].contains(&format!(
            "@bobby  {:<preview_width$}  1234 impressions  1234 likes  1234 profile visits",
            "longer text"
        )));
        assert!(lines[1].contains(&format!(
            "@al     {:<preview_width$}     5 impressions     5 likes     5 profile visits",
            "short"
        )));
    }

    #[test]
    fn lists_newest_first_when_impressions_tie() {
        let out = Yesterday.render(
            &Dataset {
                yesterday: vec![
                    reply_at("2026-03-10T08:00:00Z", "old", "x", 1, 0, 0),
                    reply_at("2026-03-10T20:00:00Z", "new", "x", 1, 0, 0),
                ],
                ..dataset()
            },
            fixed_now(),
        );
        assert!(out.find("@new").unwrap() < out.find("@old").unwrap());
    }

    #[test]
    fn lists_highest_impressions_first() {
        let out = Yesterday.render(
            &Dataset {
                yesterday: vec![
                    reply_at("2026-03-10T20:00:00Z", "dud", "x", 5, 0, 0),
                    reply_at("2026-03-10T08:00:00Z", "star", "x", 900, 0, 0),
                ],
                ..dataset()
            },
            fixed_now(),
        );
        assert!(out.find("@star").unwrap() < out.find("@dud").unwrap());
    }

    #[test]
    fn starts_with_header_counting_replies() {
        let out = Yesterday.render(
            &Dataset {
                yesterday: vec![reply("a"), reply("b")],
                ..dataset()
            },
            fixed_now(),
        );
        assert!(out.starts_with("Yesterday · 2 replies\n"));
    }

    #[test]
    fn says_so_when_there_are_no_replies() {
        let out = Yesterday.render(&dataset(), fixed_now());
        assert_eq!(out, "Yesterday · 0 replies\nNo replies yesterday.");
    }

    #[test]
    fn shows_replies_to_go_below_the_daily_goal() {
        let out = DailyGoal.render(
            &Dataset {
                today_count: 3,
                ..dataset()
            },
            fixed_now(),
        );
        assert_eq!(out, "Today: 3 of 5 replies (2 to go)");
    }

    #[test]
    fn shows_goal_met_at_the_daily_goal() {
        let out = DailyGoal.render(
            &Dataset {
                today_count: 5,
                ..dataset()
            },
            fixed_now(),
        );
        assert_eq!(out, "Today: 5 of 5 replies (goal met!)");
    }

    #[test]
    fn shows_goal_met_above_the_daily_goal() {
        let out = DailyGoal.render(
            &Dataset {
                today_count: 7,
                ..dataset()
            },
            fixed_now(),
        );
        assert_eq!(out, "Today: 7 of 5 replies (goal met!)");
    }

    #[test]
    fn shows_replies_to_go_with_zero_replies() {
        let out = DailyGoal.render(
            &Dataset {
                today_count: 0,
                ..dataset()
            },
            fixed_now(),
        );
        assert_eq!(out, "Today: 0 of 5 replies (5 to go)");
    }

    #[test]
    fn grid_has_title_line_above_the_squares() {
        assert_eq!(GoalGrid.title(), "Last 30 days");
        assert_eq!(grid_squares(&[0, 5]), "· ■");
    }

    #[test]
    fn shows_filled_square_for_days_meeting_the_goal() {
        assert_eq!(grid_squares(&[5]), "■");
    }

    #[test]
    fn shows_dot_for_days_below_the_goal() {
        assert_eq!(grid_squares(&[4]), "·");
    }

    #[test]
    fn shows_dot_for_a_day_with_no_replies() {
        assert_eq!(grid_squares(&[0]), "·");
    }

    #[test]
    fn joins_multiple_days_with_a_single_space() {
        assert_eq!(grid_squares(&[0, 5, 6]), "· ■ ■");
    }

    #[test]
    fn says_so_when_there_is_no_account_data() {
        assert_eq!(TopAccounts.title(), "Accounts");
        let out = TopAccounts.render(&dataset(), fixed_now());
        assert_eq!(out, "No data yet.");
    }

    #[test]
    fn shows_rank_handle_average_impressions_and_reply_count() {
        let out = TopAccounts.render(
            &Dataset {
                ranks: vec![account("alice", 150.0, 2)],
                ..dataset()
            },
            fixed_now(),
        );
        assert_eq!(TopAccounts.title(), "Accounts");
        assert!(out.contains("1."));
        assert!(out.contains("@alice"));
        assert!(out.contains("150 avg impressions"));
        assert!(out.contains("2 replies"));
    }

    #[test]
    fn aligns_columns_across_accounts() {
        let out = TopAccounts.render(
            &Dataset {
                ranks: vec![account("al", 5.0, 5), account("bobby", 1234.0, 1234)],
                ..dataset()
            },
            fixed_now(),
        );
        let lines: Vec<String> = out.lines().map(strip_osc8).collect();
        assert_eq!(lines[0], "1. @al        5 avg impressions     5 replies");
        assert_eq!(lines[1], "2. @bobby  1234 avg impressions  1234 replies");
    }

    #[test]
    fn rounds_average_impressions_to_nearest_whole_number() {
        let out = TopAccounts.render(
            &Dataset {
                ranks: vec![account("alice", 12.5, 1), account("bob", 12.4, 1)],
                ..dataset()
            },
            fixed_now(),
        );
        assert!(out.contains("13 avg impressions"));
        assert!(out.contains("12 avg impressions"));
    }

    #[test]
    fn links_the_handle_with_an_osc8_hyperlink() {
        let out = TopAccounts.render(
            &Dataset {
                ranks: vec![account("alice", 150.0, 2)],
                ..dataset()
            },
            fixed_now(),
        );
        assert!(out.contains("\x1b]8;;https://x.com/alice\x1b\\@alice\x1b]8;;\x1b\\"));
    }

    #[test]
    fn preserves_list_order_as_rank() {
        let out = TopAccounts.render(
            &Dataset {
                ranks: vec![account("first", 100.0, 1), account("second", 50.0, 1)],
                ..dataset()
            },
            fixed_now(),
        );
        let stripped: Vec<String> = out.lines().map(strip_osc8).collect();
        let first = out.find("@first").unwrap();
        let second = out.find("@second").unwrap();
        assert!(first < second);
        assert!(stripped[0].starts_with("1. @first"));
        assert!(stripped[1].starts_with("2. @second"));
    }
}
