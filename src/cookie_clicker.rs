use std::path::PathBuf;

use thirtyfour::{
    error::WebDriverError, http::reqwest_async::ReqwestDriverAsync, prelude::ElementWaitable, By,
    DesiredCapabilities, GenericWebDriver, WebDriver, WebDriverCommands,
};

pub struct CookieClicker {
    driver: GenericWebDriver<ReqwestDriverAsync>,
}

#[derive(Debug)]
pub enum CookieClickerError {
    DriverError(WebDriverError),
    SaveCodeNotFound,
}

pub type CookieClickerResult<T> = Result<T, CookieClickerError>;

const COOKIE_CLICKER_BETA_URL: &str = "https://orteil.dashnet.org/cookieclicker/beta/";

impl CookieClicker {
    pub async fn new(initial_save: String) -> CookieClickerResult<Self> {
        let caps = DesiredCapabilities::chrome();
        let driver = WebDriver::new("http://localhost:4444", &caps)
            .await
            .map_err(CookieClickerError::DriverError)?;

        let mut cookie_clicker = Self { driver };

        cookie_clicker.load_beta().await?;
        cookie_clicker.load_save_code(initial_save).await?;

        Ok(cookie_clicker)
    }

    /// Load save code into the current game
    async fn load_save_code(&mut self, initial_save: String) -> CookieClickerResult<()> {
        let save_script = format!(r#"Game.localStorageSet(Game.SaveTo, "{}");"#, initial_save);

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

        println!("Page loaded");

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

    pub async fn exit(self) -> CookieClickerResult<()> {
        self.driver
            .quit()
            .await
            .map_err(CookieClickerError::DriverError)?;

        Ok(())
    }
}
