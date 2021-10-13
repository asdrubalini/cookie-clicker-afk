use std::{
    convert::{TryFrom, TryInto},
    env,
    sync::Arc,
};

use futures::StreamExt;
use telegram_bot::{Api, ChatId, Message, MessageKind, SendMessage, UserId};
use tokio::sync::Mutex;

use crate::cookie_clicker::CookieClicker;

mod commands;

type ConcurrentCookieClicker = Arc<Mutex<CookieClicker>>;

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
        match commands::handle_commands(command_data).await {
            Ok(_) => (),
            Err(error) => {
                println!("Got an error while handling a message: {:?}", error);

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

pub async fn handle_messages(api: &Api) {
    let cookie_clicker: ConcurrentCookieClicker = Arc::new(Mutex::new(
        CookieClicker::new()
            .await
            .expect("Cannot create a new CookieClicker instance"),
    ));

    let mut stream = api.stream();

    println!("Handling messages...");

    while let Some(update) = stream.next().await {
        let update = match update {
            Ok(update) => update,
            Err(error) => {
                println!("Event error: {:?}", error);
                continue;
            }
        };

        if let telegram_bot::UpdateKind::Message(message) = update.kind {
            if !is_user_admin(&message) {
                println!("Some user tried to access the bot");
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
