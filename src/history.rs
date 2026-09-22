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

    pub fn save(&self, date: NaiveDate, replies: &[Reply]) -> Result<(), HistoryError> {
        std::fs::create_dir_all(&self.dir).map_err(HistoryError::Io)?;
        let path = self.path_for(date);
        let json = serde_json::to_vec_pretty(replies).map_err(HistoryError::Json)?;
        std::fs::write(&path, json).map_err(HistoryError::Io)
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
        let dir = std::env::temp_dir()
            .join(format!("slix-history-test-{}-{name}", std::process::id()));
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
