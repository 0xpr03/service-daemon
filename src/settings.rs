use config::{Config, ConfigError, Environment, File};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Settings {
    pub web: Web,
    pub database: Database,
    pub services: Vec<Service>,
    pub security: Security,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Web {
    pub domain: String,
    pub max_session_age_secs: i64,
    pub cookie_key: Option<Vec<u8>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Security {
    pub password_min_length: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Database {
    pub url: String,
    pub password: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Service {
    pub id: usize,
    pub name: String,
    pub restart: bool,
    pub autostart: bool,
    pub enabled: bool,
    pub command: String,
    pub directory: String,
    pub args: Vec<String>,
    pub soft_stop: Option<String>,
}

impl Settings {
    pub fn new() -> Result<Self, ConfigError> {
        let mut s = Config::new();
        s.merge(File::with_name("config/default"))?;
        s.merge(Environment::with_prefix("sc"))?;
        let mut config: Self = s.try_into()?;
        config.services.retain(|s| s.enabled);
        Ok(config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use toml;

    #[test]
    fn test_new() {
        let settings = Settings::new().unwrap();
        assert_eq!(0, settings.services.len());
    }

    /// Only for toml generation
    #[test]
    #[ignore]
    fn test_serialize() {
        let settings = Settings {
            database: Database {
                url: "test url".to_owned(),
                password: "12345".to_owned(),
            },
            security: Security {
                password_min_length: 10,
            },
            web: Web {
                domain: String::from("example.com"),
                max_session_age_secs: 60,
                cookie_key: None,
            },
            services: vec![
                Service {
                    name: "some service".to_owned(),
                    autostart: true,
                    enabled: false,
                    command: "some cmd".to_owned(),
                    directory: "/foo".to_owned(),
                    soft_stop: None,
                    args: Vec::new(),
                    id: 0,
                    restart: true,
                },
                Service {
                    name: "some service2".to_owned(),
                    autostart: false,
                    enabled: false,
                    command: "some cmd2".to_owned(),
                    directory: "/foobar".to_owned(),
                    soft_stop: Some("asdf".to_owned()),
                    args: vec!["asd".to_owned(), "def".to_owned()],
                    id: 1,
                    restart: true,
                },
            ],
        };
        println!("{}", toml::to_string(&settings).unwrap());
    }
}
