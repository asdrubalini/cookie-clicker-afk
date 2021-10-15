use std::{
    collections::VecDeque,
    env,
    path::{Path, PathBuf},
};

use chrono::{DateTime, Utc};
use chrono_tz::Tz;
use log::info;
use serde::{Deserialize, Serialize};
use tokio::{
    fs::{File, OpenOptions},
    io::{AsyncReadExt, AsyncWriteExt},
};

const MAX_BACKUPS_LENGTH: usize = 512;

#[derive(Debug)]
pub enum BackupError {
    IoError(tokio::io::Error),
    SerdeError(serde_json::Error),
}

pub type BackupResult<T> = Result<T, BackupError>;

#[derive(Debug, Serialize, Deserialize)]
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

    pub fn saved_at(&self) -> String {
        let timezone: Tz = env::var("TIMEZONE")
            .expect("Missing env TIMEZONE")
            .parse()
            .expect("Invalid env TIMEZONE");

        let saved_at = self.saved_at.with_timezone(&timezone);
        format!("{:?}", saved_at)
    }
}

pub struct Backups {
    pub backups: VecDeque<Backup>,
}

impl Backups {
    /// Load backups from disk
    pub async fn from_disk() -> BackupResult<Self> {
        let data_path = env::var("PERSISTENT_DATA_PATH").expect("Missing env PERSISTENT_DATA_PATH");
        let mut data_path = PathBuf::from(data_path);
        data_path.push("saves.json");

        if !Path::new(&data_path).exists() {
            info!("Path does not exist");
            return Ok(Self {
                backups: VecDeque::with_capacity(MAX_BACKUPS_LENGTH),
            });
        }

        // Read JSON from file
        let mut file = File::open(data_path).await.map_err(BackupError::IoError)?;

        let mut backups_str = String::new();
        file.read_to_string(&mut backups_str)
            .await
            .map_err(BackupError::IoError)?;

        let backups: VecDeque<Backup> =
            serde_json::from_str(&backups_str).map_err(BackupError::SerdeError)?;

        Ok(Self { backups })
    }

    /// Add new backup item
    pub fn push(&mut self, backup: Backup) {
        if self.backups.len() == MAX_BACKUPS_LENGTH {
            self.backups.pop_back();
        }

        self.backups.push_back(backup);
    }

    /// Get latest backup, if any
    pub fn latest(&self) -> Option<&Backup> {
        self.backups.back()
    }

    /// Write backups to disk
    pub async fn flush_to_disk(self) -> BackupResult<()> {
        let data_path = env::var("PERSISTENT_DATA_PATH").expect("Missing env PERSISTENT_DATA_PATH");
        let mut data_path = PathBuf::from(data_path);
        data_path.push("saves.json");

        // Write to file
        let mut file = OpenOptions::new()
            .create(true)
            .write(true)
            .open(data_path)
            .await
            .map_err(BackupError::IoError)?;

        let backups_json = serde_json::to_string(&self.backups).map_err(BackupError::SerdeError)?;

        file.write_all(backups_json.as_bytes())
            .await
            .map_err(BackupError::IoError)?;

        Ok(())
    }
}
