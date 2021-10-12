use std::env;

use dotenv::dotenv;
use telegram_bot::Api;

mod cookie_clicker;
mod telegram;

#[tokio::main]
async fn main() {
    dotenv().ok();

    let token = env::var("TELEGRAM_BOT_TOKEN").expect("Missing env TELEGRAM_BOT_TOKEN");
    let api = Api::new(token);

    telegram::handle_messages(&api).await;
}
