use bytes::Bytes;
use telegram_bot::{InputFileUpload, SendMessage, SendPhoto};

use crate::cookie_clicker::{CookieClicker, CookieClickerError};

use super::CommandData;

#[derive(Debug)]
pub enum MessageHandlerError {
    TelegramError(telegram_bot::Error),
    CookieClicker(CookieClickerError),
    InvalidCommand,
}

type CommandHandlerResult = Result<(), MessageHandlerError>;

pub async fn handle_commands(command_data: CommandData) -> CommandHandlerResult {
    let message = command_data.message;
    let (command, additional_data) = message.split_at(
        message
            .find(" ")
            .ok_or(MessageHandlerError::InvalidCommand)?,
    );

    // New command data with additional_data instead of the full message
    let command_data = CommandData {
        message: additional_data.to_string(),
        ..command_data
    };

    match command {
        "/start" => command_start(command_data).await,
        "/screenshot" => command_screenshot(command_data).await,
        "/status" => command_status(command_data).await,
        "/stop" => command_stop(command_data).await,
        _ => Err(MessageHandlerError::InvalidCommand),
    }
}

async fn command_start(command_data: CommandData) -> CommandHandlerResult {
    let mut cookie_clicker = command_data.cookie_clicker.lock().await;

    command_data
        .api
        .send(SendMessage::new(
            command_data.chat_id,
            "Starting a new browser session",
        ))
        .await
        .map_err(MessageHandlerError::TelegramError)?;

    command_data
        .api
        .send(SendMessage::new(
            command_data.chat_id,
            "Starting a new browser session...",
        ))
        .await
        .map_err(MessageHandlerError::TelegramError)?;

    // Start game
    cookie_clicker
        .start(command_data.message)
        .await
        .map_err(MessageHandlerError::CookieClicker)?;

    command_data.api.send(SendMessage::new(
            command_data.chat_id,
            "Browser started! Use /screenshot to get a screenshot of the current session or /status to get the status",
        ))
        .await
        .map_err(MessageHandlerError::TelegramError)?;

    Ok(())
}

async fn command_screenshot(command_data: CommandData) -> CommandHandlerResult {
    let mut cookie_clicker = command_data.cookie_clicker.lock().await;

    let screenshot = Bytes::from(
        cookie_clicker
            .take_screenshot()
            .await
            .map_err(MessageHandlerError::CookieClicker)?,
    );
    let screenshot_file = InputFileUpload::with_data(screenshot, "screenshot.png");

    command_data
        .api
        .send(SendPhoto::new(command_data.chat_id, screenshot_file))
        .await
        .map_err(MessageHandlerError::TelegramError)?;

    Ok(())
}

async fn command_status(command_data: CommandData) -> CommandHandlerResult {
    let mut cookie_clicker = command_data.cookie_clicker.lock().await;
    let cookies_count = cookie_clicker
        .get_cookies_count()
        .await
        .map_err(MessageHandlerError::CookieClicker)?;

    command_data
        .api
        .send(SendMessage::new(
            command_data.chat_id,
            format!("You have {} cookies", cookies_count),
        ))
        .await
        .map_err(MessageHandlerError::TelegramError)?;

    Ok(())
}

async fn command_stop(command_data: CommandData) -> CommandHandlerResult {
    let mut cookie_clicker = command_data.cookie_clicker.lock().await;
    let save_code = cookie_clicker
        .get_save_code()
        .await
        .map_err(MessageHandlerError::CookieClicker)?;

    let mut message = SendMessage::new(
        command_data.chat_id,
        format!(
            r#"Browser successfully stopped. Here is your code:
                <pre>{}</pre>"#,
            save_code
        ),
    );
    message.parse_mode(telegram_bot::ParseMode::Html);

    command_data
        .api
        .send(message)
        .await
        .map_err(MessageHandlerError::TelegramError)?;

    *cookie_clicker = CookieClicker::new()
        .await
        .map_err(MessageHandlerError::CookieClicker)?;

    Ok(())
}