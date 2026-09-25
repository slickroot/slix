use std::path::PathBuf;

use chrono::NaiveDate;

use crate::api::Reply;

pub struct History {
    dir: PathBuf,
}

impl History {
    pub fn new(dir: PathBuf) -> History {
        History { dir }
    }

    pub fn default_dir() -> PathBuf {
        dirs::config_dir()
            .expect("no config dir available")
            .join("slix")
            .join("history")
    }

    fn path_for(&self, date: NaiveDate) -> PathBuf {
        self.dir.join(format!("{date}.json"))
    }

    pub fn load(&self, date: NaiveDate) -> Result<Option<Vec<Reply>>, HistoryError> {
        let path = self.path_for(date);
        match std::fs::read(&path) {
            Ok(bytes) => serde_json::from_slice(&bytes)
                .map(Some)
                .map_err(HistoryError::Json),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(HistoryError::Io(e)),
        }
    }

    pub fn load_all(&self) -> Result<Vec<Reply>, HistoryError> {
        let entries = match std::fs::read_dir(&self.dir) {
            Ok(entries) => entries,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(vec![]),
            Err(e) => return Err(HistoryError::Io(e)),
        };

        let mut dates = Vec::new();
        for entry in entries {
            let entry = entry.map_err(HistoryError::Io)?;
            let path = entry.path();
            if path.extension().and_then(|ext| ext.to_str()) != Some("json") {
                continue;
            }
            let Some(stem) = path.file_stem().and_then(|stem| stem.to_str()) else {
                continue;
            };
            if let Ok(date) = stem.parse::<NaiveDate>() {
                dates.push(date);
            }
        }
        dates.sort();

        let mut replies = Vec::new();
        for date in dates {
            if let Some(mut date_replies) = self.load(date)? {
                replies.append(&mut date_replies);
            }
        }
        Ok(replies)
    }

    pub fn last_30_day_counts(&self, today: NaiveDate) -> Result<Vec<u64>, HistoryError> {
        let mut counts = Vec::with_capacity(30);
        for days_ago in (0..30).rev() {
            let date = today - chrono::Duration::days(days_ago);
            let count = self.load(date)?.unwrap_or_default().len() as u64;
            counts.push(count);
        }
        Ok(counts)
    }

    pub fn save(&self, date: NaiveDate, replies: &[Reply]) -> Result<(), HistoryError> {
        std::fs::create_dir_all(&self.dir).map_err(HistoryError::Io)?;
        let path = self.path_for(date);
        let temp_path = self
            .dir
            .join(format!("{date}.json.tmp-{}", std::process::id()));
        let json = serde_json::to_vec_pretty(replies).map_err(HistoryError::Json)?;
        std::fs::write(&temp_path, json).map_err(HistoryError::Io)?;
        std::fs::rename(&temp_path, &path).map_err(HistoryError::Io)
    }
}

impl Default for History {
    fn default() -> History {
        History::new(History::default_dir())
    }
}

#[derive(Debug)]
pub enum HistoryError {
    Io(std::io::Error),
    Json(serde_json::Error),
}

impl std::fmt::Display for HistoryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(e) => write!(f, "io error: {e}"),
            Self::Json(e) => write!(f, "history parse error: {e}"),
        }
    }
}

impl std::error::Error for HistoryError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(e) => Some(e),
            Self::Json(e) => Some(e),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_dir(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("slix-history-test-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn sample_reply(id: &str) -> Reply {
        use chrono::{TimeZone, Utc};
        Reply {
            id: id.into(),
            created_at: Utc.with_ymd_and_hms(2026, 9, 20, 10, 0, 0).unwrap(),
            text: "hi".into(),
            to_username: "bob".into(),
            impressions: 1,
            likes: 2,
            profile_visits: 3,
        }
    }

    #[test]
    fn load_for_date_with_no_file_returns_none() {
        let history = History::new(test_dir("missing"));
        let date = NaiveDate::from_ymd_opt(2026, 9, 20).unwrap();

        let loaded = history.load(date).unwrap();

        assert!(loaded.is_none());
    }

    #[test]
    fn load_for_corrupt_file_returns_err() {
        let dir = test_dir("corrupt");
        let history = History::new(dir.clone());
        let date = NaiveDate::from_ymd_opt(2026, 9, 20).unwrap();
        std::fs::write(dir.join("2026-09-20.json"), "not json").unwrap();

        let result = history.load(date);

        assert!(matches!(result, Err(HistoryError::Json(_))), "{result:?}");
    }

    #[test]
    fn saving_empty_list_round_trips_to_empty_vec() {
        let history = History::new(test_dir("empty"));
        let date = NaiveDate::from_ymd_opt(2026, 9, 20).unwrap();

        history.save(date, &[]).unwrap();
        let loaded = history.load(date).unwrap();

        assert!(matches!(loaded, Some(replies) if replies.is_empty()));
    }

    #[test]
    fn default_dir_is_config_dir_joined_with_slix_history() {
        let expected = dirs::config_dir().unwrap().join("slix").join("history");
        assert_eq!(History::default_dir(), expected);
    }

    #[test]
    fn default_uses_default_dir() {
        let history = History::default();
        assert_eq!(history.dir, History::default_dir());
    }

    #[test]
    fn save_leaves_no_stray_temp_file_behind() {
        let dir = test_dir("atomic");
        let history = History::new(dir.clone());
        let date = NaiveDate::from_ymd_opt(2026, 9, 20).unwrap();

        history.save(date, &[sample_reply("1")]).unwrap();

        let entries: Vec<_> = std::fs::read_dir(&dir)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().into_string().unwrap())
            .collect();
        assert_eq!(entries, vec!["2026-09-20.json".to_string()]);
    }

    #[test]
    fn load_all_with_no_history_dir_returns_empty() {
        let dir = test_dir("load-all-missing");
        std::fs::remove_dir_all(&dir).unwrap();
        let history = History::new(dir);

        let replies = history.load_all().unwrap();

        assert!(replies.is_empty());
    }

    #[test]
    fn load_all_with_empty_history_dir_returns_empty() {
        let history = History::new(test_dir("load-all-empty-dir"));

        let replies = history.load_all().unwrap();

        assert!(replies.is_empty());
    }

    #[test]
    fn load_all_concatenates_replies_in_date_ascending_order() {
        let history = History::new(test_dir("load-all-multi"));
        let day1 = NaiveDate::from_ymd_opt(2026, 9, 18).unwrap();
        let day2 = NaiveDate::from_ymd_opt(2026, 9, 19).unwrap();
        let day3 = NaiveDate::from_ymd_opt(2026, 9, 20).unwrap();

        history.save(day3, &[sample_reply("3")]).unwrap();
        history.save(day1, &[sample_reply("1")]).unwrap();
        history.save(day2, &[sample_reply("2")]).unwrap();

        let replies = history.load_all().unwrap();

        assert_eq!(
            replies.iter().map(|r| r.id.as_str()).collect::<Vec<_>>(),
            vec!["1", "2", "3"]
        );
    }

    #[test]
    fn load_all_propagates_err_for_corrupt_file() {
        let dir = test_dir("load-all-corrupt");
        let history = History::new(dir.clone());
        std::fs::write(dir.join("2026-09-20.json"), "not json").unwrap();

        let result = history.load_all();

        assert!(matches!(result, Err(HistoryError::Json(_))), "{result:?}");
    }

    #[test]
    fn last_30_day_counts_has_30_entries_oldest_first_today_last() {
        let history = History::new(test_dir("last-30-basic"));
        let today = NaiveDate::from_ymd_opt(2026, 9, 26).unwrap();
        let oldest = today - chrono::Duration::days(29);
        history.save(oldest, &[sample_reply("1")]).unwrap();
        history
            .save(today, &[sample_reply("2"), sample_reply("3")])
            .unwrap();

        let counts = history.last_30_day_counts(today).unwrap();

        assert_eq!(counts.len(), 30);
        assert_eq!(counts.first(), Some(&1));
        assert_eq!(counts.last(), Some(&2));
    }

    #[test]
    fn last_30_day_counts_defaults_to_zero_for_days_with_no_file() {
        let history = History::new(test_dir("last-30-empty"));
        let today = NaiveDate::from_ymd_opt(2026, 9, 26).unwrap();

        let counts = history.last_30_day_counts(today).unwrap();

        assert!(counts.iter().all(|&count| count == 0));
    }

    #[test]
    fn last_30_day_counts_reflects_replies_saved_for_a_middle_day() {
        let history = History::new(test_dir("last-30-middle"));
        let today = NaiveDate::from_ymd_opt(2026, 9, 26).unwrap();
        let middle_day = today - chrono::Duration::days(10);
        let replies = vec![sample_reply("1"), sample_reply("2"), sample_reply("3")];
        history.save(middle_day, &replies).unwrap();

        let counts = history.last_30_day_counts(today).unwrap();

        assert_eq!(counts[19], replies.len() as u64);
    }

    #[test]
    fn save_then_load_round_trips_replies_for_a_date() {
        let history = History::new(test_dir("roundtrip"));
        let date = NaiveDate::from_ymd_opt(2026, 9, 20).unwrap();
        let replies = vec![sample_reply("1"), sample_reply("2")];

        history.save(date, &replies).unwrap();
        let loaded = history.load(date).unwrap().unwrap();

        assert_eq!(loaded.len(), 2);
        assert_eq!(loaded[0].id, "1");
        assert_eq!(loaded[1].id, "2");
    }
}
