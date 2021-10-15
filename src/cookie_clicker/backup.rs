use std::{
    collections::VecDeque,
    env,
    path::{Path, PathBuf},
};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::{
    fs::OpenOptions,
    io::{AsyncReadExt, AsyncWriteExt},
};

const MAX_BACKUPS_LENGTH: usize = 512;

#[derive(Debug)]
pub enum BackupError {
    IoError(tokio::io::Error),
    SerdeError(serde_json::Error),
}

pub type BackupResult<T> = Result<T, BackupError>;

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
    backups: VecDeque<Backup>,
}

impl Backups {
    /// Load backups from disk
    pub async fn from_disk() -> BackupResult<Self> {
        let data_path = env::var("PERSISTENT_DATA_PATH").expect("Missing env PERSISTENT_DATA_PATH");
        let mut data_path = PathBuf::from(data_path);
        data_path.push("saves.json");

        if !Path::new(&data_path).exists() {
            return Ok(Self {
                backups: VecDeque::with_capacity(MAX_BACKUPS_LENGTH),
            });
        }

        // Append backup as JSON to file
        let mut file = OpenOptions::new()
            .write(false)
            .create(false)
            .open(data_path)
            .await
            .map_err(BackupError::IoError)?;

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

        // Append backup as JSON to file
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .append(true)
            .open(data_path)
            .await
            .map_err(BackupError::IoError)?;

        let backups_json =
            serde_json::to_string(&self.backups).map_err(BackupError::SerdeError)? + "\n";

        file.write_all(backups_json.as_bytes())
            .await
            .map_err(BackupError::IoError)?;

        Ok(())
    }
}
