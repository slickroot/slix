use std::collections::HashMap;

use crate::api::Reply;

pub struct AccountRank {
    pub handle: String,
    pub avg_impressions: f64,
    pub reply_count: usize,
}

pub fn rank(replies: &[Reply]) -> Vec<AccountRank> {
    let mut order: Vec<String> = Vec::new();
    let mut groups: HashMap<String, (String, u64, usize)> = HashMap::new();

    for reply in replies {
        let key = reply.to_username.to_lowercase();
        let entry = groups.entry(key.clone()).or_insert_with(|| {
            order.push(key.clone());
            (reply.to_username.clone(), 0, 0)
        });
        entry.1 += reply.impressions;
        entry.2 += 1;
    }

    let mut ranks: Vec<AccountRank> = order
        .into_iter()
        .map(|key| {
            let (handle, total_impressions, reply_count) = groups.remove(&key).unwrap();
            AccountRank {
                handle,
                avg_impressions: total_impressions as f64 / reply_count as f64,
                reply_count,
            }
        })
        .collect();

    ranks.sort_by(|a, b| b.avg_impressions.partial_cmp(&a.avg_impressions).unwrap());
    ranks.truncate(10);
    ranks
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};

    fn reply(to_username: &str, impressions: u64) -> Reply {
        Reply {
            id: "1".to_string(),
            created_at: Utc.with_ymd_and_hms(2026, 9, 20, 10, 0, 0).unwrap(),
            text: "hi".to_string(),
            to_username: to_username.to_string(),
            impressions,
            likes: 0,
            profile_visits: 0,
        }
    }

    #[test]
    fn groups_and_averages_impressions_for_same_handle() {
        let replies = vec![reply("alice", 100), reply("alice", 200)];

        let ranks = rank(&replies);

        assert_eq!(ranks.len(), 1);
        assert_eq!(ranks[0].handle, "alice");
        assert_eq!(ranks[0].avg_impressions, 150.0);
        assert_eq!(ranks[0].reply_count, 2);
    }

    #[test]
    fn groups_case_insensitively_but_displays_first_seen_casing() {
        let replies = vec![reply("Bob", 100), reply("bob", 200), reply("BOB", 300)];

        let ranks = rank(&replies);

        assert_eq!(ranks.len(), 1);
        assert_eq!(ranks[0].handle, "Bob");
        assert_eq!(ranks[0].avg_impressions, 200.0);
        assert_eq!(ranks[0].reply_count, 3);
    }

    #[test]
    fn sorts_by_average_impressions_descending() {
        let replies = vec![reply("low", 10), reply("high", 100), reply("mid", 50)];

        let ranks = rank(&replies);

        let handles: Vec<&str> = ranks.iter().map(|r| r.handle.as_str()).collect();
        assert_eq!(handles, vec!["high", "mid", "low"]);
    }

    #[test]
    fn stable_sort_keeps_first_seen_order_for_ties() {
        let replies = vec![reply("first", 50), reply("second", 50), reply("third", 50)];

        let ranks = rank(&replies);

        let handles: Vec<&str> = ranks.iter().map(|r| r.handle.as_str()).collect();
        assert_eq!(handles, vec!["first", "second", "third"]);
    }

    #[test]
    fn truncates_to_top_10_when_more_than_10_distinct_accounts() {
        let replies: Vec<Reply> = (0..15)
            .map(|i| reply(&format!("account{i}"), (15 - i) as u64))
            .collect();

        let ranks = rank(&replies);

        assert_eq!(ranks.len(), 10);
        assert_eq!(ranks[0].handle, "account0");
        assert_eq!(ranks[9].handle, "account9");
    }

    #[test]
    fn empty_input_returns_empty_output() {
        let ranks = rank(&[]);

        assert!(ranks.is_empty());
    }

    #[test]
    fn reply_count_is_correct_per_account() {
        let replies = vec![
            reply("alice", 10),
            reply("bob", 20),
            reply("alice", 30),
            reply("alice", 40),
        ];

        let ranks = rank(&replies);

        let alice = ranks.iter().find(|r| r.handle == "alice").unwrap();
        let bob = ranks.iter().find(|r| r.handle == "bob").unwrap();
        assert_eq!(alice.reply_count, 3);
        assert_eq!(bob.reply_count, 1);
    }
}
