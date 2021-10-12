use std::path::PathBuf;

use chrono::{DateTime, Utc};
use thirtyfour::{
    error::WebDriverError, http::reqwest_async::ReqwestDriverAsync, prelude::ElementWaitable, By,
    DesiredCapabilities, GenericWebDriver, WebDriver, WebDriverCommands,
};

pub struct CookieClicker {
    driver: GenericWebDriver<ReqwestDriverAsync>,
    started_at: DateTime<Utc>,
}

#[derive(Debug)]
pub enum CookieClickerError {
    DriverError(WebDriverError),
    SaveCodeNotFound,
    CookieCountNotFound,
}

pub type CookieClickerResult<T> = Result<T, CookieClickerError>;

const COOKIE_CLICKER_BETA_URL: &str = "https://orteil.dashnet.org/cookieclicker/beta/";

impl CookieClicker {
    /// Create a new `CookieClicker` object
    pub async fn new(initial_save: String) -> CookieClickerResult<Self> {
        let mut caps = DesiredCapabilities::chrome();
        caps.add_chrome_arg("--window-size=1920,1080")
            .map_err(CookieClickerError::DriverError)?;

        let driver = WebDriver::new("http://localhost:4444", &caps)
            .await
            .map_err(CookieClickerError::DriverError)?;

        let mut cookie_clicker = Self {
            driver,
            started_at: Utc::now(),
        };

        cookie_clicker.load_beta().await?;

        cookie_clicker.load_save_code(initial_save).await?;
        cookie_clicker.load_beta().await?;

        Ok(cookie_clicker)
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
    /// TODO: complete
    pub async fn take_screenshot(&mut self) -> CookieClickerResult<()> {
        self.driver
            .screenshot(&PathBuf::from("./screenshot.png"))
            .await
            .map_err(CookieClickerError::DriverError)?;

        Ok(())
    }

    /// Gets cookie count as beautified string
    pub async fn get_cookies_count(&mut self) -> CookieClickerResult<String> {
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

    pub async fn exit(self) -> CookieClickerResult<()> {
        self.driver
            .quit()
            .await
            .map_err(CookieClickerError::DriverError)?;

        Ok(())
    }
}
