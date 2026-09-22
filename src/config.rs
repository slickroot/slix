use std::fs;
use std::io::{self, Write};
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct Config {
    pub consumer_key: String,
    pub consumer_secret: String,
    pub access_token: Option<String>,
    pub access_token_secret: Option<String>,
    #[serde(default = "default_data_dir")]
    pub data_dir: PathBuf,
}

fn default_data_dir() -> PathBuf {
    dirs::config_dir()
        .expect("no config dir available")
        .join("slix")
}

#[derive(Debug)]
pub enum ConfigError {
    Io(io::Error),
    Json(serde_json::Error),
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(e) => write!(f, "io error: {e}"),
            Self::Json(e) => write!(f, "config parse error: {e}"),
        }
    }
}

impl std::error::Error for ConfigError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(e) => Some(e),
            Self::Json(e) => Some(e),
        }
    }
}

impl Config {
    pub fn path() -> PathBuf {
        dirs::config_dir()
            .expect("no config dir available")
            .join("slix")
            .join("config.json")
    }

    pub fn load() -> Result<Config, ConfigError> {
        Self::load_from(&Self::path())
    }

    pub fn load_from(path: &Path) -> Result<Config, ConfigError> {
        match fs::read(path) {
            Ok(bytes) => serde_json::from_slice(&bytes).map_err(ConfigError::Json),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(Config::default()),
            Err(e) => Err(ConfigError::Io(e)),
        }
    }

    pub fn save_to(&self, path: &Path) -> Result<(), io::Error> {
        let parent = path.parent().ok_or_else(|| {
            io::Error::new(io::ErrorKind::NotFound, "no parent dir for config path")
        })?;
        fs::create_dir_all(parent)?;
        let json = serde_json::to_vec_pretty(self).map_err(io::Error::other)?;
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .mode(0o600)
            .open(path)?;
        file.write_all(&json)
    }

    pub fn is_connected(&self) -> bool {
        self.access_token.is_some() && self.access_token_secret.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_dir(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("slix-config-test-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn load_from_missing_file_returns_unconnected() {
        let path = test_dir("missing").join("config.json");
        let cfg = Config::load_from(&path).unwrap();
        assert!(cfg.access_token.is_none());
        assert!(cfg.access_token_secret.is_none());
        assert!(!cfg.is_connected());
    }

    #[test]
    fn save_then_load_round_trips_all_fields() {
        let dir = test_dir("roundtrip");
        let path = dir.join("config.json");
        let cfg = Config {
            consumer_key: "test-consumer-key".into(),
            consumer_secret: "test-consumer-secret".into(),
            access_token: Some("test-access-token".into()),
            access_token_secret: Some("test-access-token-secret".into()),
            data_dir: dir.join("data"),
        };
        cfg.save_to(&path).unwrap();
        let loaded = Config::load_from(&path).unwrap();
        assert_eq!(loaded, cfg);
    }

    #[test]
    fn load_from_config_without_data_dir_falls_back_to_computed_default() {
        let path = test_dir("no-data-dir").join("config.json");
        fs::write(
            &path,
            r#"{
                "consumer_key": "k",
                "consumer_secret": "s",
                "access_token": null,
                "access_token_secret": null
            }"#,
        )
        .unwrap();
        let cfg = Config::load_from(&path).unwrap();
        let expected = dirs::config_dir().unwrap().join("slix");
        assert_eq!(cfg.data_dir, expected);
    }

    #[test]
    #[cfg(unix)]
    fn save_writes_file_with_mode_0600() {
        use std::os::unix::fs::PermissionsExt;
        let path = test_dir("mode").join("config.json");
        let cfg = Config {
            consumer_key: "k".into(),
            consumer_secret: "s".into(),
            access_token: Some("t".into()),
            access_token_secret: Some("ts".into()),
            ..Default::default()
        };
        cfg.save_to(&path).unwrap();
        let mode = fs::metadata(&path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600);
    }

    #[test]
    fn is_connected_requires_both_tokens() {
        let only_access_token = Config {
            consumer_key: "k".into(),
            consumer_secret: "s".into(),
            access_token: Some("t".into()),
            access_token_secret: None,
            ..Default::default()
        };
        let only_access_token_secret = Config {
            consumer_key: "k".into(),
            consumer_secret: "s".into(),
            access_token: None,
            access_token_secret: Some("ts".into()),
            ..Default::default()
        };
        let neither = Config {
            consumer_key: "k".into(),
            consumer_secret: "s".into(),
            access_token: None,
            access_token_secret: None,
            ..Default::default()
        };
        assert!(!only_access_token.is_connected());
        assert!(!only_access_token_secret.is_connected());
        assert!(!neither.is_connected());
    }

    #[test]
    fn path_is_config_dir_joined_with_slix_config_json() {
        let expected = dirs::config_dir().unwrap().join("slix").join("config.json");
        assert_eq!(Config::path(), expected);
    }
}
