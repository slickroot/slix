use std::path::PathBuf;

use chrono::{DateTime, Local, Utc};
use serde::{Deserialize, Serialize};

pub struct TodayGoal {
    path: PathBuf,
}

#[derive(Serialize, Deserialize)]
struct StoredGoal {
    count: u64,
    fetched_at: DateTime<Utc>,
}

impl TodayGoal {
    pub fn new(path: PathBuf) -> TodayGoal {
        TodayGoal { path }
    }

    pub fn load(&self, now: DateTime<Local>) -> Result<Option<u64>, TodayGoalError> {
        match std::fs::read(&self.path) {
            Ok(bytes) => {
                let stored: StoredGoal =
                    serde_json::from_slice(&bytes).map_err(TodayGoalError::Json)?;
                if is_stale(&stored, now) {
                    Ok(None)
                } else {
                    Ok(Some(stored.count))
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(TodayGoalError::Io(e)),
        }
    }

    pub fn save(&self, count: u64, now: DateTime<Local>) -> Result<(), TodayGoalError> {
        let dir = self
            .path
            .parent()
            .expect("today goal path must have a parent directory");
        std::fs::create_dir_all(dir).map_err(TodayGoalError::Io)?;
        let temp_path = dir.join(format!(
            "{}.tmp-{}",
            self.path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("today.json"),
            std::process::id()
        ));
        let stored = StoredGoal {
            count,
            fetched_at: now.with_timezone(&Utc),
        };
        let json = serde_json::to_vec_pretty(&stored).map_err(TodayGoalError::Json)?;
        std::fs::write(&temp_path, json).map_err(TodayGoalError::Io)?;
        std::fs::rename(&temp_path, &self.path).map_err(TodayGoalError::Io)
    }
}

fn is_stale(stored: &StoredGoal, now: DateTime<Local>) -> bool {
    let fetched_at_local = stored.fetched_at.with_timezone(&Local);
    if fetched_at_local.date_naive() != now.date_naive() {
        return true;
    }
    now.signed_duration_since(fetched_at_local) > chrono::Duration::hours(1)
}

#[derive(Debug)]
pub enum TodayGoalError {
    Io(std::io::Error),
    Json(serde_json::Error),
}

impl std::fmt::Display for TodayGoalError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(e) => write!(f, "io error: {e}"),
            Self::Json(e) => write!(f, "today goal parse error: {e}"),
        }
    }
}

impl std::error::Error for TodayGoalError {
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
    use chrono::TimeZone;

    fn test_dir(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("slix-today-test-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn save_then_load_round_trips_count() {
        let goal = TodayGoal::new(test_dir("roundtrip").join("today.json"));
        let now = Local.with_ymd_and_hms(2026, 9, 20, 10, 0, 0).unwrap();

        goal.save(5, now).unwrap();
        let loaded = goal.load(now).unwrap();

        assert_eq!(loaded, Some(5));
    }

    #[test]
    fn load_with_no_file_returns_none() {
        let goal = TodayGoal::new(test_dir("missing").join("today.json"));
        let now = Local.with_ymd_and_hms(2026, 9, 20, 10, 0, 0).unwrap();

        let loaded = goal.load(now).unwrap();

        assert!(loaded.is_none());
    }

    #[test]
    fn load_for_corrupt_file_returns_err() {
        let dir = test_dir("corrupt");
        let path = dir.join("today.json");
        std::fs::write(&path, "not json").unwrap();
        let goal = TodayGoal::new(path);
        let now = Local.with_ymd_and_hms(2026, 9, 20, 10, 0, 0).unwrap();

        let result = goal.load(now);

        assert!(matches!(result, Err(TodayGoalError::Json(_))), "{result:?}");
    }

    #[test]
    fn load_returns_none_when_more_than_an_hour_old() {
        let dir = test_dir("stale-by-hour");
        let path = dir.join("today.json");
        let fetched_at = Utc.with_ymd_and_hms(2026, 9, 20, 8, 0, 0).unwrap();
        let stored = StoredGoal {
            count: 3,
            fetched_at,
        };
        std::fs::write(&path, serde_json::to_vec(&stored).unwrap()).unwrap();
        let goal = TodayGoal::new(path);
        let now = fetched_at.with_timezone(&Local) + chrono::Duration::hours(2);

        let loaded = goal.load(now).unwrap();

        assert!(loaded.is_none());
    }

    #[test]
    fn load_returns_none_when_local_date_differs_even_if_recent() {
        let dir = test_dir("stale-by-day");
        let path = dir.join("today.json");
        let fetched_at_local = Local.with_ymd_and_hms(2026, 9, 20, 23, 59, 0).unwrap();
        let stored = StoredGoal {
            count: 3,
            fetched_at: fetched_at_local.with_timezone(&Utc),
        };
        std::fs::write(&path, serde_json::to_vec(&stored).unwrap()).unwrap();
        let goal = TodayGoal::new(path);
        let now = Local.with_ymd_and_hms(2026, 9, 21, 0, 5, 0).unwrap();

        let loaded = goal.load(now).unwrap();

        assert!(loaded.is_none());
    }

    #[test]
    fn load_returns_count_when_fresh() {
        let dir = test_dir("fresh");
        let path = dir.join("today.json");
        let fetched_at_local = Local.with_ymd_and_hms(2026, 9, 20, 9, 0, 0).unwrap();
        let stored = StoredGoal {
            count: 7,
            fetched_at: fetched_at_local.with_timezone(&Utc),
        };
        std::fs::write(&path, serde_json::to_vec(&stored).unwrap()).unwrap();
        let goal = TodayGoal::new(path);
        let now = Local.with_ymd_and_hms(2026, 9, 20, 9, 30, 0).unwrap();

        let loaded = goal.load(now).unwrap();

        assert_eq!(loaded, Some(7));
    }

    #[test]
    fn save_leaves_no_stray_temp_file_behind() {
        let dir = test_dir("atomic");
        let path = dir.join("today.json");
        let goal = TodayGoal::new(path);
        let now = Local.with_ymd_and_hms(2026, 9, 20, 10, 0, 0).unwrap();

        goal.save(4, now).unwrap();

        let entries: Vec<_> = std::fs::read_dir(&dir)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().into_string().unwrap())
            .collect();
        assert_eq!(entries, vec!["today.json".to_string()]);
    }
}
