use crate::accounts;
use crate::api::Reply;
use chrono::TimeZone;

const PREVIEW_LIMIT: usize = 50;
const PREVIEW_WIDTH: usize = PREVIEW_LIMIT + 1;
const DAILY_GOAL: u64 = 5;
pub const GRID_TITLE: &str = "Last 30 days";

pub fn render_today(count: u64) -> String {
    if count >= DAILY_GOAL {
        format!("Today: {count} of {DAILY_GOAL} replies (goal met!)")
    } else {
        format!(
            "Today: {count} of {DAILY_GOAL} replies ({} to go)",
            DAILY_GOAL - count
        )
    }
}

pub fn render_grid(counts: &[u64]) -> String {
    let squares = counts
        .iter()
        .map(|&count| if count >= DAILY_GOAL { "■" } else { "·" })
        .collect::<Vec<_>>()
        .join(" ");
    format!("{GRID_TITLE}\n{squares}")
}

pub fn render<Tz: TimeZone>(replies: &[Reply], tz: &Tz) -> String
where
    Tz::Offset: std::fmt::Display,
{
    let header = format!("Yesterday · {} replies", replies.len());
    if replies.is_empty() {
        return format!("{header}\nNo replies yesterday.");
    }
    let mut newest_first: Vec<&Reply> = replies.iter().collect();
    newest_first.sort_by_key(|reply| std::cmp::Reverse(reply.created_at));
    let widths = Widths::of(&newest_first);
    let lines = newest_first
        .iter()
        .map(|reply| render_line(reply, tz, &widths));
    std::iter::once(header)
        .chain(lines)
        .collect::<Vec<_>>()
        .join("\n")
}

struct Widths {
    handle: usize,
    impressions: usize,
    likes: usize,
    profile_visits: usize,
}

impl Widths {
    fn of(replies: &[&Reply]) -> Self {
        Widths {
            handle: replies
                .iter()
                .map(|reply| reply.to_username.chars().count() + 1)
                .max()
                .unwrap_or(0),
            impressions: replies
                .iter()
                .map(|reply| reply.impressions.to_string().len())
                .max()
                .unwrap_or(0),
            likes: replies
                .iter()
                .map(|reply| reply.likes.to_string().len())
                .max()
                .unwrap_or(0),
            profile_visits: replies
                .iter()
                .map(|reply| reply.profile_visits.to_string().len())
                .max()
                .unwrap_or(0),
        }
    }
}

fn render_line<Tz: TimeZone>(reply: &Reply, tz: &Tz, widths: &Widths) -> String
where
    Tz::Offset: std::fmt::Display,
{
    let time = reply.created_at.with_timezone(tz).format("%H:%M");
    let handle = format!("@{}", reply.to_username);
    let handle_width = widths.handle;
    let impressions_width = widths.impressions;
    let likes_width = widths.likes;
    let profile_visits_width = widths.profile_visits;
    format!(
        "{time}  {handle:<handle_width$}  {:<PREVIEW_WIDTH$}  {:>impressions_width$} impressions  {:>likes_width$} likes  {:>profile_visits_width$} profile visits  {}",
        preview(&reply.text),
        reply.impressions,
        reply.likes,
        reply.profile_visits,
        link(&reply.id)
    )
}

fn preview(text: &str) -> String {
    let single_line = text.replace(['\r', '\n'], " ");
    if single_line.chars().count() > PREVIEW_LIMIT {
        let cut: String = single_line.chars().take(PREVIEW_LIMIT).collect();
        format!("{cut}…")
    } else {
        single_line
    }
}

fn link(id: &str) -> String {
    format!("\x1b]8;;https://x.com/i/status/{id}\x1b\\[link]\x1b]8;;\x1b\\")
}

pub fn render_accounts(ranks: &[accounts::AccountRank]) -> String {
    let header = "Accounts";
    if ranks.is_empty() {
        return format!("{header}\nNo data yet.");
    }
    let widths = AccountWidths::of(ranks);
    let lines = ranks
        .iter()
        .enumerate()
        .map(|(index, account)| render_account_line(index + 1, account, &widths));
    std::iter::once(header.to_string())
        .chain(lines)
        .collect::<Vec<_>>()
        .join("\n")
}

struct AccountWidths {
    rank: usize,
    handle: usize,
    avg_impressions: usize,
    reply_count: usize,
}

impl AccountWidths {
    fn of(ranks: &[accounts::AccountRank]) -> Self {
        AccountWidths {
            rank: ranks.len().to_string().len(),
            handle: ranks
                .iter()
                .map(|account| account.handle.chars().count() + 1)
                .max()
                .unwrap_or(0),
            avg_impressions: ranks
                .iter()
                .map(|account| (account.avg_impressions.round() as i64).to_string().len())
                .max()
                .unwrap_or(0),
            reply_count: ranks
                .iter()
                .map(|account| account.reply_count.to_string().len())
                .max()
                .unwrap_or(0),
        }
    }
}

fn render_account_line(
    rank: usize,
    account: &accounts::AccountRank,
    widths: &AccountWidths,
) -> String {
    let rank_width = widths.rank;
    let avg_width = widths.avg_impressions;
    let reply_width = widths.reply_count;
    let handle_text = format!("@{}", account.handle);
    let handle_padding = " ".repeat(widths.handle.saturating_sub(handle_text.chars().count()));
    format!(
        "{rank:>rank_width$}. {}{handle_padding}  {:>avg_width$} avg impressions  {:>reply_width$} replies",
        account_link(&account.handle),
        account.avg_impressions.round() as i64,
        account.reply_count
    )
}

fn account_link(handle: &str) -> String {
    format!("\x1b]8;;https://x.com/{handle}\x1b\\@{handle}\x1b]8;;\x1b\\")
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{DateTime, FixedOffset, Utc};

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

    fn tz() -> FixedOffset {
        FixedOffset::east_opt(2 * 3600).unwrap()
    }

    #[test]
    fn shows_local_time_handle_and_impressions() {
        let out = render(&[reply("hi")], &tz());
        assert!(out.contains("01:30"));
        assert!(out.contains("@alice"));
        assert!(out.contains("42 impressions"));
    }

    #[test]
    fn collapses_newlines_in_preview() {
        let out = render(&[reply("a\nb\r\nc")], &tz());
        assert!(out.contains("a b  c"));
        assert_eq!(out.lines().count(), 2);
    }

    #[test]
    fn keeps_preview_at_the_limit() {
        let text = "x".repeat(PREVIEW_LIMIT);
        let out = render(&[reply(&text)], &tz());
        assert!(out.contains(&text));
        assert!(!out.contains('…'));
    }

    #[test]
    fn cuts_preview_over_the_limit_with_ellipsis() {
        let text = "x".repeat(PREVIEW_LIMIT + 1);
        let out = render(&[reply(&text)], &tz());
        assert!(out.contains(&format!("{}…", "x".repeat(PREVIEW_LIMIT))));
        assert!(!out.contains(&text));
    }

    #[test]
    fn links_with_osc8_hyperlink() {
        let out = render(&[reply("hi")], &tz());
        assert!(out.ends_with("\x1b]8;;https://x.com/i/status/123\x1b\\[link]\x1b]8;;\x1b\\"));
    }

    fn reply_with_likes(likes: u64) -> Reply {
        Reply {
            likes,
            ..reply("hi")
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

    fn reply_with_profile_visits(profile_visits: u64) -> Reply {
        Reply {
            profile_visits,
            ..reply("hi")
        }
    }

    #[test]
    fn shows_zero_profile_visits() {
        let out = render(&[reply_with_profile_visits(0)], &tz());
        assert!(out.contains("0 profile visits"));
    }

    #[test]
    fn uses_the_word_visits_for_a_single_profile_visit() {
        let out = render(&[reply_with_profile_visits(1)], &tz());
        assert!(out.contains("1 profile visits"));
    }

    #[test]
    fn puts_profile_visits_between_likes_and_link() {
        let out = render(&[reply_with_profile_visits(7)], &tz());
        let likes = out.find("0 likes").unwrap();
        let visits = out.find("7 profile visits").unwrap();
        let link = out.find("[link]").unwrap();
        assert!(likes < visits && visits < link);
    }

    #[test]
    fn shows_zero_likes() {
        let out = render(&[reply_with_likes(0)], &tz());
        assert!(out.contains("0 likes"));
    }

    #[test]
    fn uses_the_word_likes_for_a_single_like() {
        let out = render(&[reply_with_likes(1)], &tz());
        assert!(out.contains("1 likes"));
    }

    #[test]
    fn puts_likes_between_impressions_and_link() {
        let out = render(&[reply_with_likes(7)], &tz());
        let impressions = out.find("42 impressions").unwrap();
        let likes = out.find("7 likes").unwrap();
        let link = out.find("[link]").unwrap();
        assert!(impressions < likes && likes < link);
    }

    #[test]
    fn aligns_columns_across_replies() {
        let out = render(
            &[
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
            &tz(),
        );
        let preview_width = PREVIEW_WIDTH;
        let lines: Vec<&str> = out.lines().skip(1).collect();
        assert!(lines[0].contains(&format!(
            "@al     {:<preview_width$}     5 impressions     5 likes     5 profile visits",
            "short"
        )));
        assert!(lines[1].contains(&format!(
            "@bobby  {:<preview_width$}  1234 impressions  1234 likes  1234 profile visits",
            "longer text"
        )));
    }

    #[test]
    fn lists_newest_first() {
        let out = render(
            &[
                reply_at("2026-03-10T08:00:00Z", "old", "x", 1, 0, 0),
                reply_at("2026-03-10T20:00:00Z", "new", "x", 1, 0, 0),
            ],
            &tz(),
        );
        assert!(out.find("@new").unwrap() < out.find("@old").unwrap());
    }

    #[test]
    fn starts_with_header_counting_replies() {
        let out = render(&[reply("a"), reply("b")], &tz());
        assert!(out.starts_with("Yesterday · 2 replies\n"));
    }

    #[test]
    fn says_so_when_there_are_no_replies() {
        assert_eq!(
            render(&[], &tz()),
            "Yesterday · 0 replies\nNo replies yesterday."
        );
    }

    #[test]
    fn shows_replies_to_go_below_the_daily_goal() {
        assert_eq!(
            render_today(3),
            format!("Today: 3 of {DAILY_GOAL} replies (2 to go)")
        );
    }

    #[test]
    fn shows_goal_met_at_the_daily_goal() {
        assert_eq!(
            render_today(DAILY_GOAL),
            format!("Today: {DAILY_GOAL} of {DAILY_GOAL} replies (goal met!)")
        );
    }

    #[test]
    fn shows_goal_met_above_the_daily_goal() {
        assert_eq!(
            render_today(7),
            format!("Today: 7 of {DAILY_GOAL} replies (goal met!)")
        );
    }

    #[test]
    fn shows_replies_to_go_with_zero_replies() {
        assert_eq!(
            render_today(0),
            format!("Today: 0 of {DAILY_GOAL} replies ({DAILY_GOAL} to go)")
        );
    }

    fn grid_squares(counts: &[u64]) -> String {
        render_grid(counts).lines().nth(1).unwrap().to_string()
    }

    #[test]
    fn grid_has_title_line_above_the_squares() {
        let out = render_grid(&[0, DAILY_GOAL]);
        assert_eq!(out.lines().next(), Some(GRID_TITLE));
        assert_eq!(out.lines().count(), 2);
    }

    #[test]
    fn shows_filled_square_for_days_meeting_the_goal() {
        assert_eq!(grid_squares(&[DAILY_GOAL]), "■");
    }

    #[test]
    fn shows_dot_for_days_below_the_goal() {
        assert_eq!(grid_squares(&[DAILY_GOAL - 1]), "·");
    }

    #[test]
    fn shows_dot_for_a_day_with_no_replies() {
        assert_eq!(grid_squares(&[0]), "·");
    }

    #[test]
    fn joins_multiple_days_with_a_single_space() {
        assert_eq!(grid_squares(&[0, DAILY_GOAL, DAILY_GOAL + 1]), "· ■ ■");
    }

    fn account(handle: &str, avg_impressions: f64, reply_count: usize) -> accounts::AccountRank {
        accounts::AccountRank {
            handle: handle.into(),
            avg_impressions,
            reply_count,
        }
    }

    #[test]
    fn says_so_when_there_is_no_account_data() {
        assert_eq!(render_accounts(&[]), "Accounts\nNo data yet.");
    }

    #[test]
    fn shows_rank_handle_average_impressions_and_reply_count() {
        let out = render_accounts(&[account("alice", 150.0, 2)]);
        assert!(out.starts_with("Accounts\n"));
        assert!(out.contains("1."));
        assert!(out.contains("@alice"));
        assert!(out.contains("150 avg impressions"));
        assert!(out.contains("2 replies"));
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
    fn aligns_columns_across_accounts() {
        let out = render_accounts(&[account("al", 5.0, 5), account("bobby", 1234.0, 1234)]);
        let lines: Vec<String> = out.lines().skip(1).map(strip_osc8).collect();
        assert_eq!(lines[0], "1. @al        5 avg impressions     5 replies");
        assert_eq!(lines[1], "2. @bobby  1234 avg impressions  1234 replies");
    }

    #[test]
    fn rounds_average_impressions_to_nearest_whole_number() {
        let out = render_accounts(&[account("alice", 12.5, 1), account("bob", 12.4, 1)]);
        assert!(out.contains("13 avg impressions"));
        assert!(out.contains("12 avg impressions"));
    }

    #[test]
    fn links_the_handle_with_an_osc8_hyperlink() {
        let out = render_accounts(&[account("alice", 150.0, 2)]);
        assert!(out.contains("\x1b]8;;https://x.com/alice\x1b\\@alice\x1b]8;;\x1b\\"));
    }

    #[test]
    fn preserves_list_order_as_rank() {
        let out = render_accounts(&[account("first", 100.0, 1), account("second", 50.0, 1)]);
        let stripped: Vec<String> = out.lines().map(strip_osc8).collect();
        let first = out.find("@first").unwrap();
        let second = out.find("@second").unwrap();
        assert!(first < second);
        assert!(stripped[1].starts_with("1. @first"));
        assert!(stripped[2].starts_with("2. @second"));
    }
}
