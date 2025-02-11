pub mod config;
pub mod handler;
pub mod dto;
pub mod module;
pub mod constants;

use core::panic;
use tokio_tungstenite::connect_async;
use futures::StreamExt;
use redis::Client;
use log::{info,error};
use log::LevelFilter;


#[tokio::main]
async fn main() -> std::io::Result<()> {
    let config = config::init_config().await;

    env_logger::builder().filter_level(LevelFilter::Debug).init();
    
    // Set owner ID at startup
    constants::set_owner_id(config.bot.owner);

    // Set AI configuration
    constants::set_ai_token(config.ai.token);
    constants::set_ai_endpoint(config.ai.endpoint);
    constants::set_ai_default_model(config.ai.default_model);
    constants::set_ai_init_prompt(config.ai.init_prompt);
    constants::set_ai_engage_time(config.ai.engage_time);

    // Initialize SQLite database connection
    let db = Client::open(config.redis.url).unwrap();

    let addr = config.api.url + "/?access_token=" + &config.api.access_token;

    let (socket, response) = connect_async(addr).await.unwrap();

    info!("Connected to the server: {:?}", response);

    let (sender, mut receiver) = socket.split();

    let arc_sender = std::sync::Arc::new(tokio::sync::Mutex::new(sender));
    let arc_db = std::sync::Arc::new(db);

    loop {
        let msg = receiver.next().await.unwrap().unwrap();
        if msg.is_text() {
            let msg = msg.to_text().unwrap().to_string();
            let sender_clone = arc_sender.clone();
            let db_clone = arc_db.clone();
            
            tokio::spawn(async move {
                if let Err(e) = handler::recv(&msg, sender_clone, db_clone).await {
                    error!("Thread error: {:?}", e);
                }
            });
        } else {
            panic!("Received a non-text message");
        }
    }
}