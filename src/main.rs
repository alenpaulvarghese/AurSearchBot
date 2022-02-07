use std::sync::Arc;
use std::time::Duration;

use reqwest::Client;
use retainer::Cache;
use teloxide::{dispatching2::UpdateFilterExt, prelude2::*};

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
    let utils = Arc::new(Utils {
        cache: Arc::clone(&cache),
        client: Client::new(),
    });

    let utils_ref = Arc::clone(&utils);
    tokio::spawn(async move { cache.monitor(4, 0.25, Duration::from_secs(15)).await });
    let handler = Update::filter_message().branch(dptree::endpoint(message_handler));
    let inline_handler =
        Update::filter_inline_query().branch(dptree::endpoint(inline_queries_handler));
    Dispatcher::builder(bot, handler.chain(inline_handler))
        .dependencies(dptree::deps![utils_ref])
        .build()
        .setup_ctrlc_handler()
        .dispatch()
        .await;
}
