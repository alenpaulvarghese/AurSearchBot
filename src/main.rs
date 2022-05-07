use std::sync::Arc;
use std::time::Duration;

use retainer::Cache;
use teloxide::dptree::endpoint;
use teloxide::prelude::*;

use handlers::{inline_queries_handler, message_handler};
use request::{AurResponse, Search, Utils};

mod handlers;
mod request;

#[tokio::main]
async fn main() {
    log4rs::init_file("log.yaml", Default::default()).unwrap();
    run().await;
}

async fn run() {
    log::info!("Starting bot...");
    let bot = Bot::from_env().auto_send();
    let cache: Arc<Cache<Search, AurResponse>> = Arc::new(Cache::new());
    let utils = Arc::new(Utils::new(&cache));

    tokio::spawn(async move { cache.monitor(4, 0.25, Duration::from_secs(15)).await });

    let inline_handler = Update::filter_inline_query().branch(endpoint(inline_queries_handler));
    let message_handler = Update::filter_message().branch(endpoint(message_handler));

    let handler = dptree::entry()
        .branch(message_handler)
        .branch(inline_handler);
    Dispatcher::builder(bot, handler)
        .dependencies(dptree::deps![utils])
        .build()
        .setup_ctrlc_handler()
        .dispatch()
        .await;
}
