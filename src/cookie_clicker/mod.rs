use std::{env, num::ParseFloatError};

use log::{info, trace};
use thirtyfour::{
    error::WebDriverError, http::reqwest_async::ReqwestDriverAsync, prelude::ElementWaitable, By,
    DesiredCapabilities, GenericWebDriver, WebDriver, WebDriverCommands,
};

mod tasks;
pub use tasks::{ConcurrentCookieClicker, CookieClickerTasks};

mod backup;
pub use backup::{Backup, BackupError, Backups};

type Driver = GenericWebDriver<ReqwestDriverAsync>;

#[derive(Debug)]
pub struct CookieClicker {
    driver: Option<Driver>,
    pub backups: Backups,
}

#[derive(Debug)]
pub enum CookieClickerError {
    DriverError(WebDriverError),
    SaveCodeNotFound,
    CookieCountNotFound,
    IoError(tokio::io::Error),
    ParseFloat(ParseFloatError),
    DriverNotStarted,
    BackupError(BackupError),
}

pub type CookieClickerResult<T> = Result<T, CookieClickerError>;

const COOKIE_CLICKER_BETA_URL: &str = "https://orteil.dashnet.org/cookieclicker/beta/";

impl CookieClicker {
    /// Create a new `CookieClicker` object
    pub fn new() -> CookieClickerResult<Self> {
        let backups = Backups::new().map_err(CookieClickerError::BackupError)?;

        Ok(Self {
            driver: None,
            backups,
        })
    }

    /// Start the actual cookie clicker session
    pub async fn start(&mut self, initial_save: String) -> CookieClickerResult<()> {
        self.connect().await?;
        self.load_beta().await?;
        self.load_save_code(initial_save).await?;
        self.load_beta().await?;

        Ok(())
    }

    /// Connect to Selenium instance
    async fn connect(&mut self) -> CookieClickerResult<()> {
        let mut caps = DesiredCapabilities::chrome();
        caps.add_chrome_arg("--window-size=1920,1080")
            .map_err(CookieClickerError::DriverError)?;

        let driver_url = env::var("DRIVER_URL").expect("Missing env DRIVER_URL");

        trace!("Connecting to {}", driver_url);

        let driver = WebDriver::new(&driver_url, &caps)
            .await
            .map_err(CookieClickerError::DriverError)?;

        trace!("Connected");

        self.driver = Some(driver);

        Ok(())
    }

    /// Get driver instance or fail if it is not initialized
    pub fn driver(&self) -> CookieClickerResult<&Driver> {
        if self.driver.is_none() {
            Err(CookieClickerError::DriverNotStarted)
        } else {
            Ok(self.driver.as_ref().unwrap())
        }
    }

    pub fn is_started(&self) -> bool {
        self.driver.is_some()
    }

    /// Retrieve save code and store on disk for later use
    pub async fn backup_save_code(&mut self) -> CookieClickerResult<()> {
        let backup = Backup::new(self.get_save_code().await?);
        self.backups
            .add(backup)
            .map_err(CookieClickerError::BackupError)?;

        Ok(())
    }

    /// Load save code into the current game
    async fn load_save_code(&mut self, initial_save: String) -> CookieClickerResult<()> {
        let driver = self.driver()?;

        let save_script = format!(
            r#"
            while (typeof Game.localStorageSet !== "function");
            return Game.localStorageSet(Game.SaveTo, "{}");
            "#,
            initial_save
        );

        trace!("Loading save code...");

        driver
            .execute_script(&save_script)
            .await
            .map_err(CookieClickerError::DriverError)?;

        trace!("Save code loaded");

        Ok(())
    }

    pub async fn get_save_code(&mut self) -> CookieClickerResult<String> {
        let driver = self.driver()?;

        let save_code = driver
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
        let driver = self.driver()?;

        while !driver
            .execute_script("return document.readyState")
            .await
            .map_err(CookieClickerError::DriverError)?
            .value()
            .to_string()
            .contains("complete")
        {}

        let big_cookie = driver
            .find_element(By::Id("bigCookie"))
            .await
            .map_err(CookieClickerError::DriverError)?;

        big_cookie.wait_until();

        Ok(())
    }

    /// Hide some stuff from gui
    async fn prepare_gui(&mut self) -> CookieClickerResult<()> {
        let driver = self.driver()?;

        let prepare_script = r#"
            document.getElementById('smallSupport').style.display = 'none';
            document.getElementById('topBar').style.display = 'none';
            document.getElementById('game').style.top = '0px';
            document.getElementsByClassName('cc_banner')[0].style.display = 'none';
        "#;

        driver
            .execute_script(prepare_script)
            .await
            .map_err(CookieClickerError::DriverError)?;

        Ok(())
    }

    /// Navigate to the beta page of the game
    async fn load_beta(&mut self) -> CookieClickerResult<()> {
        let driver = self.driver()?;

        trace!("Loading beta...");

        driver
            .get(COOKIE_CLICKER_BETA_URL)
            .await
            .map_err(CookieClickerError::DriverError)?;

        self.wait_page_load().await?;

        trace!("Loaded");

        self.prepare_gui().await?;

        Ok(())
    }

    /// Take a screenshot of the current page
    pub async fn take_screenshot(&mut self) -> CookieClickerResult<Vec<u8>> {
        let driver = self.driver()?;

        let screenshot = driver
            .screenshot_as_png()
            .await
            .map_err(CookieClickerError::DriverError)?;

        Ok(screenshot)
    }

    /// Gets cookie count
    pub async fn get_cookies_count(&mut self) -> CookieClickerResult<f64> {
        let driver = self.driver()?;

        let cookies_count = driver
            .execute_script("return Game.cookies")
            .await
            .map_err(CookieClickerError::DriverError)?
            .value()
            .as_f64()
            .ok_or(CookieClickerError::CookieCountNotFound)?;

        Ok(cookies_count)
    }

    /// Gets cookies per second
    pub async fn get_cookies_per_second(&mut self) -> CookieClickerResult<f64> {
        let driver = self.driver()?;

        let cookies_count = driver
            .execute_script("return Game.cookiesPs * (1 - Game.cpsSucked)")
            .await
            .map_err(CookieClickerError::DriverError)?
            .value()
            .as_f64()
            .ok_or(CookieClickerError::CookieCountNotFound)?;

        Ok(cookies_count)
    }

    /// Get the beautified cookies count
    pub async fn beautify_cookies(&mut self, cookies: f64) -> CookieClickerResult<String> {
        let driver = self.driver()?;
        let script = format!("return Beautify({})", cookies);

        let cookies_count = driver
            .execute_script(&script)
            .await
            .map_err(CookieClickerError::DriverError)?
            .value()
            .as_str()
            .ok_or(CookieClickerError::CookieCountNotFound)?
            .to_string();

        Ok(cookies_count)
    }

    pub async fn exit(&mut self) -> CookieClickerResult<()> {
        let driver = self
            .driver
            .take()
            .ok_or(CookieClickerError::DriverNotStarted)?;

        info!("Quitting...");

        driver
            .quit()
            .await
            .map_err(CookieClickerError::DriverError)?;

        Ok(())
    }
}
