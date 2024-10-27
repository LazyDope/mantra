//! This module provides configuration data and serialization
use std::{
    fs::File,
    io::{Seek, SeekFrom},
};

use serde::{Deserialize, Serialize};
use thiserror::Error;
use time::UtcOffset;

#[cfg(doc)]
use crate::app::App;

mod config_serde;

/// Possible errors while loading a [`Config`] from a file
#[derive(Error, Debug)]
pub enum ConfigError {
    #[error(transparent)]
    BaseDirs(#[from] xdg::BaseDirectoriesError),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Serde(#[from] serde_yaml::Error),
}

/// Configuration for the [`App`] to use
#[derive(Serialize, Deserialize)]
pub struct Config {
    pub currency: Currency,
    #[serde(with = "config_serde::utc_offset")]
    pub timezone: UtcOffset,
}

/// Configuration for currency type, optional short form
#[derive(Serialize, Deserialize)]
pub struct Currency {
    pub long: String,
    pub short: Option<String>,
}

impl Config {
    /// A default configuration for Manna and local/UTC time (current_local_offset doesn't work on linux)
    pub fn new() -> Self {
        Self {
            currency: "Manna".into(),
            timezone: UtcOffset::current_local_offset().unwrap_or(UtcOffset::UTC),
        }
    }

    /// Loads or creates a default config in the mantra xdg directory
    pub async fn load_or_create() -> Result<Config, ConfigError> {
        let config_path = super::base_dirs()?.place_config_file("config.yaml")?;
        let config_file = match File::open(&config_path) {
            Ok(file) => file,
            Err(error) => match error.kind() {
                std::io::ErrorKind::NotFound => {
                    let mut file = std::fs::OpenOptions::new()
                        .read(true)
                        .write(true)
                        .create_new(true)
                        .open(&config_path)?;
                    serde_yaml::to_writer(&file, &Config::default())?;
                    file.seek(SeekFrom::Start(0))
                        .expect("Seek to the start of a file we just created cannot fail");
                    file
                }
                _ => return Err(error.into()),
            },
        };
        Ok(serde_yaml::from_reader(config_file)?)
    }
}

impl Default for Config {
    fn default() -> Self {
        Self::new()
    }
}

impl From<String> for Currency {
    fn from(value: String) -> Self {
        Self {
            long: value,
            short: None,
        }
    }
}

impl From<&str> for Currency {
    fn from(value: &str) -> Self {
        Self::from(value.to_owned())
    }
}
