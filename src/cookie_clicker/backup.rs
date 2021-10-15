use std::{env, path::Path};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::{
    fs::OpenOptions,
    io::{AsyncReadExt, AsyncWriteExt},
};

use super::{CookieClickerError, CookieClickerResult};

const MAX_BACKUPS_LENGTH: usize = 512;

#[derive(Serialize, Deserialize)]
pub struct Backup {
    saved_at: DateTime<Utc>,
    pub save_code: String,
}

impl Backup {
    /// Create new `CookieClickerBackup` instance
    pub fn new(save_code: String) -> Self {
        Self {
            saved_at: Utc::now(),
            save_code,
        }
    }
}

pub struct Backups {
    backups: Vec<Backup>,
}

impl Backups {
    /// Load backups from disk
    pub async fn from_disk() -> CookieClickerResult<Self> {
        let data_path = env::var("PERSISTENT_DATA_PATH").expect("Missing env PERSISTENT_DATA_PATH");

        if !Path::new(&data_path).exists() {
            return Ok(Self {
                backups: Vec::new(),
            });
        }

        // Append backup as JSON to file
        let mut file = OpenOptions::new()
            .write(false)
            .create(false)
            .open(data_path)
            .await
            .map_err(CookieClickerError::IoError)?;

        let mut backups_str = String::new();
        file.read_to_string(&mut backups_str)
            .await
            .map_err(CookieClickerError::IoError)?;

        let backups: Vec<Backup> =
            serde_json::from_str(&backups_str).map_err(CookieClickerError::SerdeError)?;

        Ok(Self { backups })
    }

    /// Add new backup item
    pub fn push(&mut self, backup: Backup) {
        if self.backups.len() == MAX_BACKUPS_LENGTH {
            self.backups.remove(0);
        }

        self.push(backup);
    }

    /// Get latest backup, if any
    pub fn latest(&self) -> Option<&Backup> {
        self.backups.last()
    }

    /// Write backups to disk
    pub async fn flush_to_disk(self) -> CookieClickerResult<()> {
        let data_path = env::var("PERSISTENT_DATA_PATH").expect("Missing env PERSISTENT_DATA_PATH");

        // Append backup as JSON to file
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .append(true)
            .open(data_path)
            .await
            .map_err(CookieClickerError::IoError)?;

        let backups_json =
            serde_json::to_string(&self.backups).map_err(CookieClickerError::SerdeError)? + "\n";

        file.write_all(backups_json.as_bytes())
            .await
            .map_err(CookieClickerError::IoError)?;

        Ok(())
    }
}
