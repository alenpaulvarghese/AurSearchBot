mod request;

use request::{search, AurResponse};
use std::time::Duration;

use reqwest::Client;
use retainer::{entry::CacheEntryReadGuard, Cache};
use std::{error::Error, sync::Arc};
use teloxide::{
    payloads::AnswerInlineQuerySetters,
    prelude::*,
    types::{
        InlineQueryResult, InlineQueryResultArticle, InputMessageContent, InputMessageContentText,
        ParseMode,
    },
    utils::command::BotCommand,
};
use tokio_stream::wrappers::UnboundedReceiverStream;

struct Utils {
    cache: Arc<Cache<String, AurResponse>>,
    client: Client,
}

#[derive(BotCommand)]
#[command(rename = "lowercase", description = "These commands are supported:")]
enum Command {
    #[command(description = "display this text.")]
    Help,
    #[command(description = "search a package.")]
    Search(String),
}

async fn cached_search<'a>(
    utils: &'a Utils,
    query: &String,
) -> CacheEntryReadGuard<'a, AurResponse> {
    if let Some(cache) = utils.cache.get(query).await {
        cache
    } else {
        let response = search(&utils.client, query).await;
        utils
            .cache
            .insert(query.clone(), response.clone(), Duration::from_secs(30))
            .await;
        utils.cache.get(query).await.unwrap()
    }
}

async fn inline_queries_handler(
    cx: UpdateWithCx<AutoSend<Bot>, InlineQuery>,
    utils: Arc<Utils>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    // check if the query is empty
    if cx.update.query.is_empty() {
        cx.requester
            .answer_inline_query(cx.update.id, [])
            .switch_pm_text("Type to search packages on AUR")
            .switch_pm_parameter("start")
            .await?;
        return Ok(());
    }
    let mut inline_result: Vec<InlineQueryResult> = Vec::new();
    let mut offset = cx.update.offset.parse::<usize>().unwrap_or(0);
    let aur_response = cached_search(&utils, &cx.update.query).await;
    if let AurResponse::Result { results, .. } = &*aur_response {
        let mut end = offset + 50;
        if end > results.len() {
            end = results.len()
        }
        for items in &results[offset..end] {
            inline_result.push(InlineQueryResult::Article(
                InlineQueryResultArticle::new(
                    items.id.clone().to_string(),
                    items.name.clone(),
                    InputMessageContent::Text(
                        InputMessageContentText::new(&items.pretty())
                            .parse_mode(ParseMode::Html)
                            .disable_web_page_preview(true),
                    ),
                )
                .description(&items.description),
            ));
        }
        offset = if offset + 50 < results.len() {
            offset + 50
        } else {
            0
        };
    } else if let AurResponse::Error { error } = &*aur_response {
        inline_result.push(InlineQueryResult::Article(InlineQueryResultArticle::new(
            "1",
            error,
            InputMessageContent::Text(InputMessageContentText::new(
                "Error occured while searching AUR",
            )),
        )))
    }
    let mut req_builder = cx
        .requester
        .answer_inline_query(cx.update.id, inline_result)
        .cache_time(10);
    if offset != 0 {
        req_builder = req_builder.next_offset(offset.to_string());
    }
    req_builder.await?;

    Ok(())
}

async fn message_handler(
    cx: UpdateWithCx<AutoSend<Bot>, Message>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let text = cx.update.text();
    if let None = text {
        return Ok(());
    }
    match Command::parse(text.unwrap(), "Click counter") {
        Ok(command) => {
            match command {
                Command::Help => {
                    cx.answer(Command::descriptions()).await?;
                }
                Command::Search(string) => {
                    if string.is_empty() {
                        cx.reply_to("Please provid a search phrase").await?;
                    } else {
                        cx.reply_to("currently not implemented").await?;
                    }
                }
            };
        }
        Err(_) => {}
    };

    Ok(())
}

#[tokio::main]
async fn main() {
    teloxide::enable_logging!();
    log::info!("Starting  bot...");

    let bot = Bot::from_env().auto_send();
    let cache: Arc<Cache<String, AurResponse>> = Arc::new(Cache::new());
    let utils = Utils {
        cache: Arc::clone(&cache),
        client: Client::new(),
    };
    let utils_ref = Arc::new(utils);
    tokio::spawn(async move { cache.monitor(4, 0.25, Duration::from_secs(3)).await });
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
        .dispatch()
        .await;
}
