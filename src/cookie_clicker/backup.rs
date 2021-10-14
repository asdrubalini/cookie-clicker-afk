use std::env;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::{fs::OpenOptions, io::AsyncWriteExt};

use super::{CookieClickerError, CookieClickerResult};

#[derive(Serialize, Deserialize)]
pub struct CookieClickerBackup {
    saved_at: DateTime<Utc>,
    save_code: String,
}

impl CookieClickerBackup {
    /// Create new `CookieClickerBackup` instance
    pub fn new(save_code: String) -> Self {
        Self {
            saved_at: Utc::now(),
            save_code,
        }
    }

    /// Write backup to disk
    pub async fn write(self) -> CookieClickerResult<()> {
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
            serde_json::to_string(&self).map_err(CookieClickerError::SerdeError)? + "\n";

        file.write_all(backups_json.as_bytes())
            .await
            .map_err(CookieClickerError::IoError)?;

        Ok(())
    }
}
