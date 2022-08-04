use crate::db::models::SID;
use config::{Config, ConfigError, Environment, File};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::PathBuf;

#[derive(thiserror::Error, Debug)]
pub enum SettingsError {
    #[error("config error: {0}")]
    ParsingError(ConfigError),
    #[error("The service id '{0}' is used multiple times!")]
    IDReuse(SID),
}

impl From<ConfigError> for SettingsError {
    fn from(error: ConfigError) -> Self {
        SettingsError::ParsingError(error)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Settings {
    pub web: Web,
    pub database: Option<Database>,
    pub services: Vec<Service>,
    pub security: Security,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Web {
    pub max_session_age_secs: u32,
    pub bind_ip: String,
    pub bind_port: u16,
    pub cookie_secure: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Security {
    pub password_min_length: usize,
    pub bcrypt_cost: u32,
    pub disable_totp: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Database {
    pub url: String,
    pub password: String,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct Service {
    pub id: SID,
    pub name: String,
    #[serde(default)]
    pub restart: bool,
    #[serde(default)]
    pub autostart: bool,
    pub enabled: bool,
    #[serde(default)]
    pub allow_relative: bool,
    pub command: String,
    pub directory: PathBuf,
    #[serde(default)]
    pub args: Vec<String>,
    pub soft_stop: Option<String>,
    #[serde(default)]
    pub restart_always: bool,
    #[serde(default)]
    pub snapshot_console_on_stop: bool,
    #[serde(default = "default_true")]
    pub snapshot_console_on_crash: bool,
    #[serde(default)]
    pub snapshot_console_on_manual_stop: bool,
    #[serde(default)]
    pub snapshot_console_on_manual_kill: bool,
    #[serde(default)]
    pub retry_max: Option<usize>,
    #[serde(default)]
    pub retry_backoff_ms: Option<u64>,
}

impl Settings {
    pub fn new() -> Result<Self, SettingsError> {
        Self::new_opt(None)
    }
    pub fn new_opt(file: Option<&str>) -> Result<Self, SettingsError> {
        let mut s = Config::builder();
        if let Some(f) = file {
            s = s.add_source(File::with_name(f));
        } else {
            s = s
                .add_source(File::with_name("config/services"))
                .add_source(Environment::with_prefix("sd"));
        }
        let config = s.build()?;
        let mut config: Self = config.try_deserialize()?;

        config.validate()?;

        config.services.retain(|s| s.enabled);
        Ok(config)
    }
    fn validate(&self) -> Result<(), SettingsError> {
        let mut ids = HashSet::new();
        for service in self.services.iter() {
            if !ids.insert(service.id) {
                return Err(SettingsError::IDReuse(service.id));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use log::trace;
    use toml;

    #[test]
    fn test_id_reuse() {
        Settings::new_opt(Some("tests/double_id.valid.toml")).unwrap();

        match Settings::new_opt(Some("tests/double_id.toml")) {
            Err(SettingsError::IDReuse(id)) => assert_eq!(1, id),
            v => panic!("Expected IDReuse error got {:?}", v),
        }
    }

    #[test]
    fn test_new() {
        let settings = Settings::new_opt(Some("config/template.toml")).unwrap();
        assert_eq!(4, settings.services.len());
    }

    /// Only for toml generation
    #[test]
    #[ignore]
    fn test_serialize() {
        let settings = Settings {
            database: Some(Database {
                url: "test url".to_owned(),
                password: "12345".to_owned(),
            }),
            security: Security {
                password_min_length: 10,
                bcrypt_cost: 10,
                disable_totp: false,
            },
            web: Web {
                max_session_age_secs: 60,
                bind_ip: String::from("127.0.0.1"),
                bind_port: 9000,
                cookie_secure: true,
            },
            services: vec![
                Service {
                    name: "some service".to_owned(),
                    autostart: true,
                    restart_always: false,
                    enabled: false,
                    allow_relative: true,
                    command: "some cmd".to_owned(),
                    directory: "./foo".into(),
                    soft_stop: None,
                    args: Vec::new(),
                    snapshot_console_on_stop: true,
                    snapshot_console_on_crash: true,
                    snapshot_console_on_manual_stop: true,
                    snapshot_console_on_manual_kill: true,
                    id: 0,
                    restart: true,
                    retry_backoff_ms: Some(0),
                    retry_max: Some(0),
                },
                Service {
                    name: "some service2".to_owned(),
                    autostart: false,
                    enabled: false,
                    restart_always: true,
                    allow_relative: true,
                    command: "some cmd2".to_owned(),
                    directory: "./foobar".into(),
                    snapshot_console_on_stop: true,
                    snapshot_console_on_crash: true,
                    snapshot_console_on_manual_stop: true,
                    snapshot_console_on_manual_kill: true,
                    soft_stop: Some("asdf".to_owned()),
                    args: vec!["asd".to_owned(), "def".to_owned()],
                    id: 1,
                    restart: true,
                    retry_backoff_ms: Some(0),
                    retry_max: Some(0),
                },
            ],
        };
        trace!("{}", toml::to_string(&settings).unwrap());
    }
}
