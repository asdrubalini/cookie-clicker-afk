use bytes::Bytes;
use log::info;
use telegram_bot::{InputFileUpload, SendDocument, SendMessage};

use crate::cookie_clicker::CookieClickerError;

use super::CommandData;

#[derive(Debug)]
pub enum CommandHandlerError {
    TelegramError(telegram_bot::Error),
    CookieClicker(CookieClickerError),
    InvalidCommand,
    InstanceNotStarted,
    InstanceAlreadyStarted,
    NoBackupsFound,
}

type CommandHandlerResult = Result<(), CommandHandlerError>;

pub async fn handle_command(command_data: CommandData) -> CommandHandlerResult {
    let message = command_data.message;

    let (command, additional_data) = if message.contains(' ') {
        let (command, additional_data) = message.split_at(
            message
                .find(' ')
                .ok_or(CommandHandlerError::InvalidCommand)?,
        );

        (
            command.to_string(),
            additional_data[1..additional_data.len()].to_string(),
        )
    } else {
        (message, "".to_string())
    };

    info!("Command: {} Data: {}", command, additional_data);

    // New command data with additional_data instead of the full message
    let command_data = CommandData {
        message: additional_data,
        ..command_data
    };

    match command.as_str() {
        "/start" => command_start(command_data).await,
        "/resume" => command_resume(command_data).await,
        "/screenshot" => command_screenshot(command_data).await,
        "/details" => command_details(command_data).await,
        "/backup" => command_backup(command_data).await,
        "/stop" => command_stop(command_data).await,
        _ => Err(CommandHandlerError::InvalidCommand),
    }
}

async fn command_start(command_data: CommandData) -> CommandHandlerResult {
    let mut cookie_clicker = command_data.cookie_clicker.lock().await;

    if cookie_clicker.is_started() {
        return Err(CommandHandlerError::InstanceAlreadyStarted);
    }

    command_data
        .api
        .send(SendMessage::new(
            command_data.chat_id,
            "Starting a new browser session...",
        ))
        .await
        .map_err(CommandHandlerError::TelegramError)?;

    // Start game
    cookie_clicker
        .start(command_data.message)
        .await
        .map_err(CommandHandlerError::CookieClicker)?;

    command_data.api.send(SendMessage::new(
            command_data.chat_id,
            "Browser started! Use /screenshot to get a screenshot of the current session or /details to get details",
        ))
        .await
        .map_err(CommandHandlerError::TelegramError)?;

    Ok(())
}

async fn command_resume(command_data: CommandData) -> CommandHandlerResult {
    let mut cookie_clicker = command_data.cookie_clicker.lock().await;

    if cookie_clicker.is_started() {
        return Err(CommandHandlerError::InstanceAlreadyStarted);
    }

    let backup = cookie_clicker
        .backups
        .latest_backup()
        .map_err(CookieClickerError::BackupError)
        .map_err(CommandHandlerError::CookieClicker)?;

    let backup = match backup {
        Some(backup) => backup,
        None => return Err(CommandHandlerError::NoBackupsFound),
    };

    let save_code = backup.save_code.to_owned();

    let message = format!(
        "Starting a new browser session with backup taken at {}",
        backup.saved_at()
    );

    command_data
        .api
        .send(SendMessage::new(command_data.chat_id, message))
        .await
        .map_err(CommandHandlerError::TelegramError)?;

    // Start game
    cookie_clicker
        .start(save_code)
        .await
        .map_err(CommandHandlerError::CookieClicker)?;

    command_data.api.send(SendMessage::new(
            command_data.chat_id,
            "Browser started! Use /screenshot to get a screenshot of the current session or /details to get details",
        ))
        .await
        .map_err(CommandHandlerError::TelegramError)?;

    Ok(())
}

async fn command_screenshot(command_data: CommandData) -> CommandHandlerResult {
    let mut cookie_clicker = command_data.cookie_clicker.lock().await;

    if !cookie_clicker.is_started() {
        return Err(CommandHandlerError::InstanceNotStarted);
    }

    command_data
        .api
        .send(SendMessage::new(
            command_data.chat_id,
            "Taking screenshot...",
        ))
        .await
        .map_err(CommandHandlerError::TelegramError)?;

    let screenshot = Bytes::from(
        cookie_clicker
            .take_screenshot()
            .await
            .map_err(CommandHandlerError::CookieClicker)?,
    );
    let screenshot_file = InputFileUpload::with_data(screenshot, "screenshot.png");

    command_data
        .api
        .send(SendDocument::new(command_data.chat_id, screenshot_file))
        .await
        .map_err(CommandHandlerError::TelegramError)?;

    Ok(())
}

async fn command_details(command_data: CommandData) -> CommandHandlerResult {
    let mut cookie_clicker = command_data.cookie_clicker.lock().await;

    if !cookie_clicker.is_started() {
        return Err(CommandHandlerError::InstanceNotStarted);
    }

    let cookies_count = cookie_clicker
        .get_cookies_count()
        .await
        .map_err(CommandHandlerError::CookieClicker)?;

    let cookies_count_beautified = cookie_clicker
        .beautify_cookies(cookies_count)
        .await
        .map_err(CommandHandlerError::CookieClicker)?;

    let cookies_per_hour = cookie_clicker
        .get_cookies_per_second()
        .await
        .map_err(CommandHandlerError::CookieClicker)?
        * 60.0
        * 60.0;

    let cookies_per_hour_beautified = cookie_clicker
        .beautify_cookies(cookies_per_hour)
        .await
        .map_err(CommandHandlerError::CookieClicker)?;

    let message = format!(
        "You have {} cookies and currently producing {} cookies per hour",
        cookies_count_beautified, cookies_per_hour_beautified
    );

    command_data
        .api
        .send(SendMessage::new(command_data.chat_id, message))
        .await
        .map_err(CommandHandlerError::TelegramError)?;

    Ok(())
}

async fn command_backup(command_data: CommandData) -> CommandHandlerResult {
    let mut cookie_clicker = command_data.cookie_clicker.lock().await;

    if !cookie_clicker.is_started() {
        return Err(CommandHandlerError::InstanceNotStarted);
    }

    command_data
        .api
        .send(SendMessage::new(command_data.chat_id, "Starting backup..."))
        .await
        .map_err(CommandHandlerError::TelegramError)?;

    cookie_clicker
        .backup_save_code()
        .await
        .map_err(CommandHandlerError::CookieClicker)?;

    command_data
        .api
        .send(SendMessage::new(command_data.chat_id, "Backup complete"))
        .await
        .map_err(CommandHandlerError::TelegramError)?;

    Ok(())
}

async fn command_stop(command_data: CommandData) -> CommandHandlerResult {
    let mut cookie_clicker = command_data.cookie_clicker.lock().await;

    if !cookie_clicker.is_started() {
        return Err(CommandHandlerError::InstanceNotStarted);
    }

    let save_code = cookie_clicker
        .get_save_code()
        .await
        .map_err(CommandHandlerError::CookieClicker)?;

    let first_message = SendMessage::new(
        command_data.chat_id,
        "Browser successfully stopped. Here is your code:",
    );

    let mut second_message =
        SendMessage::new(command_data.chat_id, format!("<pre>{}</pre>", save_code));
    second_message.parse_mode(telegram_bot::ParseMode::Html);

    command_data
        .api
        .send(first_message)
        .await
        .map_err(CommandHandlerError::TelegramError)?;

    command_data
        .api
        .send(second_message)
        .await
        .map_err(CommandHandlerError::TelegramError)?;

    cookie_clicker
        .exit()
        .await
        .map_err(CommandHandlerError::CookieClicker)?;

    Ok(())
}
