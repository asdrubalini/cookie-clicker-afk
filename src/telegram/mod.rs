use std::{env, sync::Arc};

use futures::StreamExt;
use log::{error, info, warn};
use telegram_bot::{Api, ChatId, Message, MessageKind, SendMessage, UserId};
use tokio::sync::Mutex;

use crate::cookie_clicker::{ConcurrentCookieClicker, CookieClicker, CookieClickerTasks};

mod commands;

pub struct CommandData {
    api: Api,
    chat_id: ChatId,
    cookie_clicker: ConcurrentCookieClicker,
    message: String,
}

impl CommandData {
    fn new(
        api: Api,
        chat_id: ChatId,
        cookie_clicker: ConcurrentCookieClicker,
        message: String,
    ) -> Self {
        Self {
            api,
            chat_id,
            cookie_clicker,
            message,
        }
    }
}

async fn command_task(api: Api, command_data: CommandData, chat_id: ChatId) {
    tokio::spawn(async move {
        match commands::handle_command(command_data).await {
            Ok(_) => (),
            Err(error) => {
                error!("Error: {:?}", error);

                let mut message =
                    SendMessage::new(chat_id, format!("Error: <pre>{:#?}</pre>", error));
                message.parse_mode(telegram_bot::ParseMode::Html);
                let _ = api.send(message).await;
            }
        }
    });
}

/// Validates user identity from message
fn is_user_admin(message: &Message) -> bool {
    let admin_id = UserId::from(
        env::var("TELEGRAM_ADMIN_ID")
            .expect("Missing env TELEGRAM_ADMIN_ID")
            .parse::<i64>()
            .expect("Invalid TELEGRAM_ADMIN_ID"),
    );

    message.from.id == admin_id
}

/// Main event handler loop
pub async fn handle_events(api: &Api) {
    let cookie_clicker: ConcurrentCookieClicker = Arc::new(Mutex::new(
        CookieClicker::new().expect("Cannot create CookieClicker instance"),
    ));

    {
        // Start async jobs
        let cookie_clicker = cookie_clicker.clone();
        CookieClickerTasks::new(cookie_clicker).start().await;
    }

    let mut stream = api.stream();

    info!("Handling messages...");

    while let Some(update) = stream.next().await {
        let update = match update {
            Ok(update) => update,
            Err(error) => {
                error!("Event error: {:?}", error);
                continue;
            }
        };

        if let telegram_bot::UpdateKind::Message(message) = update.kind {
            if !is_user_admin(&message) {
                warn!("Some user tried to access the bot");
                continue;
            }

            if let MessageKind::Text { data, .. } = message.kind {
                let api = api.clone();
                let chat_id = message.chat.id();
                let cookie_clicker = cookie_clicker.clone();

                let command_data = CommandData::new(api.clone(), chat_id, cookie_clicker, data);
                command_task(api, command_data, chat_id).await;
            }
        }
    }
}
