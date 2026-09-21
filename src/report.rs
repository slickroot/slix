use crate::api::Reply;
use chrono::TimeZone;

const PREVIEW_LIMIT: usize = 50;
const PREVIEW_WIDTH: usize = PREVIEW_LIMIT + 1;

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
}
