mod handlers;
mod request;

use handlers::{callback_handler, inline_queries_handler, message_handler};
use request::{AurResponse, Utils};

use std::sync::Arc;
use std::time::Duration;

use reqwest::Client;
use retainer::Cache;
use teloxide::prelude::*;
use tokio_stream::wrappers::UnboundedReceiverStream;

#[tokio::main]
async fn main() {
    log4rs::init_file("log.yaml", Default::default()).unwrap();
    run().await;
}

async fn run() {
    log::info!("Starting bot...");
    let bot = Bot::from_env().auto_send();
    let cache: Arc<Cache<String, AurResponse>> = Arc::new(Cache::new());
    let utils = Utils {
        cache: Arc::clone(&cache),
        client: Client::new(),
    };
    let utils_ref = Arc::new(utils);
    tokio::spawn(async move { cache.monitor(4, 0.25, Duration::from_secs(15)).await });
    Dispatcher::new(bot)
        .messages_handler(|rx: DispatcherHandlerRx<AutoSend<Bot>, Message>| {
            UnboundedReceiverStream::new(rx).for_each_concurrent(None, |cx| async move {
                message_handler(cx).await.log_on_error().await;
            })
        })
        .inline_queries_handler(|rx: DispatcherHandlerRx<AutoSend<Bot>, InlineQuery>| {
            UnboundedReceiverStream::new(rx).for_each_concurrent(None, move |cx| {
                let ref_c = Arc::clone(&utils_ref);
                async move {
                    inline_queries_handler(cx, ref_c).await.log_on_error().await;
                }
            })
        })
        .callback_queries_handler(|rx: DispatcherHandlerRx<AutoSend<Bot>, CallbackQuery>| {
            UnboundedReceiverStream::new(rx).for_each_concurrent(None, |cx| async move {
                callback_handler(cx).await.log_on_error().await;
            })
        })
        .dispatch()
        .await;
}
