use std::sync::Arc;

use futures::StreamExt;
use telegram_bot::{Api, ChatId, MessageKind, SendMessage};
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
            if let MessageKind::Text { data, .. } = message.kind {
                let api = api.clone();
                let chat_id = message.chat.id();
                let cookie_clicker = cookie_clicker.clone();

                let command_data = CommandData::new(api.clone(), chat_id, cookie_clicker, data);

                tokio::spawn(async move {
                    match commands::handle_commands(command_data).await {
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
