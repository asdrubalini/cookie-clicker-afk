use std::{env, path::PathBuf};

use chrono::{DateTime, Utc};
use chrono_tz::Tz;
use rusqlite::{params, Connection, OptionalExtension};

const MAX_BACKUPS_LENGTH: usize = 512;

#[derive(Debug)]
pub enum BackupError {
    RusqliteError(rusqlite::Error),
}

pub type BackupResult<T> = Result<T, BackupError>;

#[derive(Debug)]
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

#[derive(Debug)]
pub struct Backups {
    connection: Connection,
}

impl Backups {
    pub fn new() -> BackupResult<Self> {
        let data_path = env::var("PERSISTENT_DATA_PATH").expect("Missing env PERSISTENT_DATA_PATH");
        let mut data_path = PathBuf::from(data_path);
        data_path.push("saves.db");

        let mut backups = Self {
            connection: Connection::open(data_path).map_err(BackupError::RusqliteError)?,
        };
        backups.create_tables()?;

        Ok(backups)
    }

    fn create_tables(&mut self) -> BackupResult<()> {
        self.connection
            .execute(include_str!("./sql/schema.sql"), [])
            .map_err(BackupError::RusqliteError)?;

        Ok(())
    }

    pub fn add(&mut self, backup: Backup) -> BackupResult<()> {
        self.connection
            .execute(
                include_str!("./sql/insert_backup.sql"),
                params![backup.save_code, backup.saved_at],
            )
            .map_err(BackupError::RusqliteError)?;

        Ok(())
    }

    pub fn latest_backup(&mut self) -> BackupResult<Option<Backup>> {
        self
            .connection
            .query_row(include_str!("./sql/get_latest_backup.sql"), [], |row| {
                Ok(Backup {
                    save_code: row.get(0)?,
                    saved_at: row.get(1)?,
                })
            })
            .optional()
            .map_err(BackupError::RusqliteError)
    }
}
