use crate::accounts;
use crate::model::Reply;
use chrono::TimeZone;

pub(crate) mod table;

pub(crate) const PREVIEW_LIMIT: usize = 50;
pub(crate) const PREVIEW_WIDTH: usize = PREVIEW_LIMIT + 1;

pub(crate) struct Widths {
    handle: usize,
    impressions: usize,
    likes: usize,
    profile_visits: usize,
}

impl Widths {
    pub(crate) fn of(replies: &[&Reply]) -> Self {
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

pub(crate) fn render_line<Tz: TimeZone>(reply: &Reply, tz: &Tz, widths: &Widths) -> String
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

pub(crate) fn preview(text: &str) -> String {
    let single_line = text.replace(['\r', '\n'], " ");
    if single_line.chars().count() > PREVIEW_LIMIT {
        let cut: String = single_line.chars().take(PREVIEW_LIMIT).collect();
        format!("{cut}…")
    } else {
        single_line
    }
}

pub(crate) fn link(id: &str) -> String {
    format!("\x1b]8;;https://x.com/i/status/{id}\x1b\\[link]\x1b]8;;\x1b\\")
}

pub(crate) struct AccountWidths {
    rank: usize,
    handle: usize,
    avg_impressions: usize,
    reply_count: usize,
}

impl AccountWidths {
    pub(crate) fn of(ranks: &[accounts::AccountRank]) -> Self {
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

pub(crate) fn render_account_line(
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

pub(crate) fn account_link(handle: &str) -> String {
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
    fn collapses_newlines_in_preview() {
        let reply = reply("a\nb\r\nc");
        let out = render_line(&reply, &tz(), &Widths::of(&[&reply]));
        assert!(out.contains("a b  c"));
        assert_eq!(out.lines().count(), 1);
    }

    #[test]
    fn keeps_preview_at_the_limit() {
        let text = "x".repeat(PREVIEW_LIMIT);
        let reply = reply(&text);
        let out = render_line(&reply, &tz(), &Widths::of(&[&reply]));
        assert!(out.contains(&text));
        assert!(!out.contains('…'));
    }

    #[test]
    fn cuts_preview_over_the_limit_with_ellipsis() {
        let text = "x".repeat(PREVIEW_LIMIT + 1);
        let reply = reply(&text);
        let out = render_line(&reply, &tz(), &Widths::of(&[&reply]));
        assert!(out.contains(&format!("{}…", "x".repeat(PREVIEW_LIMIT))));
        assert!(!out.contains(&text));
    }

    #[test]
    fn links_with_osc8_hyperlink() {
        let reply = reply("hi");
        let out = render_line(&reply, &tz(), &Widths::of(&[&reply]));
        assert!(out.ends_with("\x1b]8;;https://x.com/i/status/123\x1b\\[link]\x1b]8;;\x1b\\"));
    }

    fn reply_with_likes(likes: u64) -> Reply {
        Reply {
            likes,
            ..reply("hi")
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
        let reply = reply_with_profile_visits(0);
        let out = render_line(&reply, &tz(), &Widths::of(&[&reply]));
        assert!(out.contains("0 profile visits"));
    }

    #[test]
    fn uses_the_word_visits_for_a_single_profile_visit() {
        let reply = reply_with_profile_visits(1);
        let out = render_line(&reply, &tz(), &Widths::of(&[&reply]));
        assert!(out.contains("1 profile visits"));
    }

    #[test]
    fn puts_profile_visits_between_likes_and_link() {
        let reply = reply_with_profile_visits(7);
        let out = render_line(&reply, &tz(), &Widths::of(&[&reply]));
        let likes = out.find("0 likes").unwrap();
        let visits = out.find("7 profile visits").unwrap();
        let link = out.find("[link]").unwrap();
        assert!(likes < visits && visits < link);
    }

    #[test]
    fn shows_zero_likes() {
        let reply = reply_with_likes(0);
        let out = render_line(&reply, &tz(), &Widths::of(&[&reply]));
        assert!(out.contains("0 likes"));
    }

    #[test]
    fn uses_the_word_likes_for_a_single_like() {
        let reply = reply_with_likes(1);
        let out = render_line(&reply, &tz(), &Widths::of(&[&reply]));
        assert!(out.contains("1 likes"));
    }

    #[test]
    fn puts_likes_between_impressions_and_link() {
        let reply = reply_with_likes(7);
        let out = render_line(&reply, &tz(), &Widths::of(&[&reply]));
        let impressions = out.find("42 impressions").unwrap();
        let likes = out.find("7 likes").unwrap();
        let link = out.find("[link]").unwrap();
        assert!(impressions < likes && likes < link);
    }
}
