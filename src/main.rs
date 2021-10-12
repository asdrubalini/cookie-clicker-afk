use std::{fs::File, io::Read, time::Duration};

use crate::cookie_clicker::CookieClicker;

mod cookie_clicker;

#[tokio::main]
async fn main() {
    let mut save_file = File::open("AsdrubaliniBakery.txt").unwrap();
    let mut save_code = String::new();
    save_file.read_to_string(&mut save_code).unwrap();

    let mut cookie = CookieClicker::new(save_code).await.unwrap();

    println!("{} cookies", cookie.get_cookies_count().await.unwrap());

    for i in 0..200000 {
        tokio::time::sleep(Duration::from_secs(2)).await;
        println!("{} cookies", cookie.get_cookies_count().await.unwrap());
    }

    cookie.exit().await.unwrap();
}
