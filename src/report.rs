use crate::api::Reply;
use chrono::TimeZone;

const PREVIEW_LIMIT: usize = 50;

pub fn render<Tz: TimeZone>(replies: &[Reply], tz: &Tz) -> String
where
    Tz::Offset: std::fmt::Display,
{
    replies
        .iter()
        .map(|reply| render_line(reply, tz))
        .collect::<Vec<_>>()
        .join("\n")
}

fn render_line<Tz: TimeZone>(reply: &Reply, tz: &Tz) -> String
where
    Tz::Offset: std::fmt::Display,
{
    let time = reply.created_at.with_timezone(tz).format("%H:%M");
    format!(
        "{time}  @{}  {}  {} impressions  {}",
        reply.to_username,
        preview(&reply.text),
        reply.impressions,
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
        assert!(!out.contains('\n'));
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
}
