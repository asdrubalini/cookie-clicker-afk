use std::{env, num::ParseIntError};

use chrono::{DateTime, Utc};
use log::info;
use serde::{Deserialize, Serialize};
use thirtyfour::{
    error::WebDriverError, http::reqwest_async::ReqwestDriverAsync, prelude::ElementWaitable, By,
    DesiredCapabilities, GenericWebDriver, WebDriver, WebDriverCommands,
};
use tokio::{fs::OpenOptions, io::AsyncWriteExt};

#[derive(Serialize, Deserialize)]
struct CookieClickerBackup {
    saved_at: DateTime<Utc>,
    save_code: String,
}

impl CookieClickerBackup {
    pub fn new(save_code: String) -> Self {
        Self {
            saved_at: Utc::now(),
            save_code,
        }
    }

    pub async fn write(self) -> CookieClickerResult<()> {
        let data_path = env::var("PERSISTENT_DATA_PATH").expect("Missing env PERSISTENT_DATA_PATH");

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

pub struct CookieClicker {
    driver: GenericWebDriver<ReqwestDriverAsync>,
}

#[derive(Debug)]
pub enum CookieClickerError {
    DriverError(WebDriverError),
    SaveCodeNotFound,
    CookieCountNotFound,
    IoError(tokio::io::Error),
    SerdeError(serde_json::Error),
    ParseInt(ParseIntError),
}

pub type CookieClickerResult<T> = Result<T, CookieClickerError>;

const COOKIE_CLICKER_BETA_URL: &str = "https://orteil.dashnet.org/cookieclicker/beta/";
const RESTART_TASK_SLEEP_TIME: u64 = 86400;

impl CookieClicker {
    /// Create a new `CookieClicker` object
    pub async fn new() -> CookieClickerResult<Self> {
        let mut caps = DesiredCapabilities::chrome();
        caps.add_chrome_arg("--window-size=1920,1080")
            .map_err(CookieClickerError::DriverError)?;

        let driver_url = env::var("DRIVER_URL").expect("Missing env DRIVER_URL");

        info!("Connecting to {}", driver_url);

        let driver = WebDriver::new(&driver_url, &caps)
            .await
            .map_err(CookieClickerError::DriverError)?;

        Ok(Self { driver })
    }

    /// Start the actual cookie clicker session
    pub async fn start(&mut self, initial_save: String) -> CookieClickerResult<()> {
        self.load_beta().await?;
        self.load_save_code(initial_save).await?;
        self.load_beta().await?;

        Ok(())
    }

    /// Retrieve backup code and save on disk for later use
    pub async fn backup_save_code(&mut self) -> CookieClickerResult<()> {
        let backup = CookieClickerBackup::new(self.get_save_code().await?);
        backup.write().await?;

        Ok(())
    }

    /// Load save code into the current game
    async fn load_save_code(&mut self, initial_save: String) -> CookieClickerResult<()> {
        let save_script = format!(
            r#"
            while (typeof Game.localStorageSet !== "function");
            return Game.localStorageSet(Game.SaveTo, "{}");
            "#,
            initial_save
        );

        self.driver
            .execute_script(&save_script)
            .await
            .map_err(CookieClickerError::DriverError)?;

        Ok(())
    }

    pub async fn get_save_code(&mut self) -> CookieClickerResult<String> {
        let save_code = self
            .driver
            .execute_script("return Game.localStorageGet(Game.SaveTo);")
            .await
            .map_err(CookieClickerError::DriverError)?
            .value()
            .as_str()
            .ok_or(CookieClickerError::SaveCodeNotFound)?
            .to_string();

        Ok(save_code)
    }

    /// Wait until page is loaded and the big cookie has appeared on the screen
    async fn wait_page_load(&mut self) -> CookieClickerResult<()> {
        while !self
            .driver
            .execute_script("return document.readyState")
            .await
            .map_err(CookieClickerError::DriverError)?
            .value()
            .to_string()
            .contains("complete")
        {}

        let big_cookie = self
            .driver
            .find_element(By::Id("bigCookie"))
            .await
            .map_err(CookieClickerError::DriverError)?;

        big_cookie.wait_until();

        Ok(())
    }

    /// Navigate to the beta page of the game
    async fn load_beta(&mut self) -> CookieClickerResult<()> {
        self.driver
            .get(COOKIE_CLICKER_BETA_URL)
            .await
            .map_err(CookieClickerError::DriverError)?;

        self.wait_page_load().await?;

        Ok(())
    }

    /// Take a screenshot of the current page
    pub async fn take_screenshot(&mut self) -> CookieClickerResult<Vec<u8>> {
        let screenshot = self
            .driver
            .screenshot_as_png()
            .await
            .map_err(CookieClickerError::DriverError)?;

        Ok(screenshot)
    }

    /// Gets cookie count as beautified string
    pub async fn get_pretty_cookies_count(&mut self) -> CookieClickerResult<String> {
        let cookies_count = self
            .driver
            .execute_script("return Beautify(Game.cookies)")
            .await
            .map_err(CookieClickerError::DriverError)?
            .value()
            .as_str()
            .ok_or(CookieClickerError::CookieCountNotFound)?
            .to_string();

        Ok(cookies_count)
    }

    /// Gets cookie count
    pub async fn get_cookies_count(&mut self) -> CookieClickerResult<u128> {
        let cookies_count = self
            .driver
            .execute_script("return Game.cookies")
            .await
            .map_err(CookieClickerError::DriverError)?
            .value()
            .as_str()
            .ok_or(CookieClickerError::CookieCountNotFound)?
            .to_string();

        Ok(cookies_count
            .parse::<u128>()
            .map_err(CookieClickerError::ParseInt)?)
    }

    pub async fn exit(self) -> CookieClickerResult<()> {
        info!("Quitting...");

        self.driver
            .quit()
            .await
            .map_err(CookieClickerError::DriverError)?;

        Ok(())
    }
}
