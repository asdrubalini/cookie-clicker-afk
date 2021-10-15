use std::{sync::Arc, time::Duration};

use log::{error, info};
use tokio::sync::Mutex;

use super::CookieClicker;

pub type ConcurrentCookieClicker = Arc<Mutex<CookieClicker>>;

const BACKUP_TASK_WAIT_SECONDS: u64 = 1;

pub struct CookieClickerTasks {
    cookie_clicker: ConcurrentCookieClicker,
}

impl CookieClickerTasks {
    /// Create new `CookieClickerTasks` instance
    pub fn new(cookie_clicker: ConcurrentCookieClicker) -> Self {
        Self { cookie_clicker }
    }

    /// Start tasks
    pub async fn start(self) {
        tokio::spawn(async move { Self::backup_save_code_task(self.cookie_clicker).await });
    }

    /// Perform save code backup once in a while
    async fn backup_save_code_task(cookie_clicker: ConcurrentCookieClicker) {
        loop {
            tokio::time::sleep(Duration::from_secs(BACKUP_TASK_WAIT_SECONDS)).await;

            {
                let mut cookie_clicker = cookie_clicker.lock().await;

                if !cookie_clicker.is_started() {
                    info!("CookieClicker instance is not active");
                    continue;
                }

                match cookie_clicker.backup_save_code().await {
                    Ok(_) => info!("Back up done"),
                    Err(error) => error!("There was an error while backing up: {:?}", error),
                }
            }
        }
    }
}
