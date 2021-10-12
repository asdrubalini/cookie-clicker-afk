use std::{fs::File, io::Read, time::Duration};

use crate::cookie_clicker::CookieClicker;

mod cookie_clicker;

#[tokio::main]
async fn main() {
    let mut save_file = File::open("AsdrubaliniBakery.txt").unwrap();
    let mut save_buf = String::new();
    save_file.read_to_string(&mut save_buf).unwrap();

    let mut cookie = CookieClicker::new(save_buf).await.unwrap();

    tokio::time::sleep(Duration::from_secs(2)).await;

    let new_save = cookie.get_save_code().await.unwrap();
    println!("{}", new_save);

    cookie.take_screenshot().await.unwrap();

    cookie.exit().await.unwrap();
}
