use std::sync::Arc;

use bytes::Bytes;
use futures::StreamExt;
use telegram_bot::{Api, ChatId, InputFileUpload, MessageKind, SendMessage, SendPhoto};
use tokio::sync::Mutex;

use crate::cookie_clicker::{CookieClicker, CookieClickerError};

type ConcurrentCookieClicker = Arc<Mutex<CookieClicker>>;

#[derive(Debug)]
pub enum MessageHandlerError {
    TelegramError(telegram_bot::Error),
    CookieClicker(CookieClickerError),
    InvalidCommand,
}

pub async fn handle_text_messages(
    api: &Api,
    chat_id: ChatId,
    cookie_clicker: ConcurrentCookieClicker,
    message_text: String,
) -> Result<(), MessageHandlerError> {
    if message_text.starts_with("/start ") {
        let save_code = message_text.replace("/start ", "");
        let mut cookie_clicker = cookie_clicker.lock().await;

        api.send(SendMessage::new(chat_id, "Starting a new browser session"))
            .await
            .map_err(MessageHandlerError::TelegramError)?;

        api.send(SendMessage::new(
            chat_id,
            "Starting a new browser session...",
        ))
        .await
        .map_err(MessageHandlerError::TelegramError)?;

        // Start game
        cookie_clicker
            .start(save_code)
            .await
            .map_err(MessageHandlerError::CookieClicker)?;

        api.send(SendMessage::new(
            chat_id,
            "Browser started! Use /screenshot to get a screenshot of the current session or /status to get the status",
        ))
        .await
        .map_err(MessageHandlerError::TelegramError)?;
    } else if message_text.starts_with("/screenshot") {
        let mut cookie_clicker = cookie_clicker.lock().await;

        let screenshot = Bytes::from(
            cookie_clicker
                .take_screenshot()
                .await
                .map_err(MessageHandlerError::CookieClicker)?,
        );
        let screenshot_file = InputFileUpload::with_data(screenshot, "screenshot.png");

        api.send(SendPhoto::new(chat_id, screenshot_file))
            .await
            .map_err(MessageHandlerError::TelegramError)?;
    } else if message_text.starts_with("/status") {
        let mut cookie_clicker = cookie_clicker.lock().await;
        let cookies_count = cookie_clicker
            .get_cookies_count()
            .await
            .map_err(MessageHandlerError::CookieClicker)?;

        api.send(SendMessage::new(
            chat_id,
            format!("You have {} cookies", cookies_count),
        ))
        .await
        .map_err(MessageHandlerError::TelegramError)?;
    } else if message_text.starts_with("/stop") {
        let mut cookie_clicker = cookie_clicker.lock().await;
        let save_code = cookie_clicker
            .get_save_code()
            .await
            .map_err(MessageHandlerError::CookieClicker)?;

        let mut message = SendMessage::new(
            chat_id,
            format!(
                r#"Browser successfully stopped. Here is your code:
                <pre>{}</pre>"#,
                save_code
            ),
        );
        message.parse_mode(telegram_bot::ParseMode::Html);

        api.send(message)
            .await
            .map_err(MessageHandlerError::TelegramError)?;

        *cookie_clicker = CookieClicker::new()
            .await
            .map_err(MessageHandlerError::CookieClicker)?;
    }

    Err(MessageHandlerError::InvalidCommand)
}

pub async fn handle_messages(api: &Api) {
    let cookie_clicker: ConcurrentCookieClicker =
        Arc::new(Mutex::new(CookieClicker::new().await.unwrap()));

    let mut stream = api.stream();

    println!("Handling messages...");

    while let Some(update) = stream.next().await {
        let update = update.unwrap();

        if let telegram_bot::UpdateKind::Message(message) = update.kind {
            if let MessageKind::Text { data, .. } = message.kind {
                let api = api.clone();
                let chat_id = message.chat.id();
                let cookie_clicker = cookie_clicker.clone();

                tokio::spawn(async move {
                    match handle_text_messages(&api, chat_id, cookie_clicker, data).await {
                        Ok(_) => (),
                        Err(error) => {
                            println!("Got an error while handling a message: {:?}", error);

                            let mut message = SendMessage::new(
                                chat_id,
                                format!("Error: <pre>{:#?}</pre>", error),
                            );
                            message.parse_mode(telegram_bot::ParseMode::Html);
                            let _ = api.send(message).await;
                        }
                    }
                });
            }
        }
    }
}
