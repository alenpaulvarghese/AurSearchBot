mod request;

use request::{search, AurResponse};
use reqwest::Client;
use std::error::Error;
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

#[derive(BotCommand)]
#[command(rename = "lowercase", description = "These commands are supported:")]
enum Command {
    #[command(description = "display this text.")]
    Help,
    #[command(description = "initate a new count message.")]
    Search(String),
}

async fn inline_queries_handler(
    cx: UpdateWithCx<AutoSend<Bot>, InlineQuery>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let client = Client::new();
    let mut inline_result: Vec<InlineQueryResult> = Vec::new();
    let mut offset = cx.update.offset.parse::<usize>().unwrap_or(0);

    if let AurResponse::Result { results, .. } = search(&client, &cx.update.query).await {
        let mut end = offset + 50;
        if end > results.len() {
            end = results.len()
        }
        println!(
            "current offset: {}\tcurrent end: {}\tvector length: {}",
            offset,
            end,
            results.len()
        );
        for items in &results[offset..end] {
            inline_result.push(InlineQueryResult::Article(
                InlineQueryResultArticle::new(
                    items.id.clone().to_string(),
                    items.name.clone(),
                    InputMessageContent::Text(
                        InputMessageContentText::new(&items.pretty()).parse_mode(ParseMode::Html),
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
        println!("offset changed: {}", offset);
    }

    let mut req_builder = cx
        .requester
        .answer_inline_query(cx.update.id, inline_result)
        .cache_time(10);
    if offset != 0 {
        req_builder = req_builder.next_offset(offset.to_string());
    }
    req_builder.await?;
    dbg!(cx.update.query);
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
    log::info!("Starting aur-search bot...");

    let bot = Bot::from_env().auto_send();

    Dispatcher::new(bot)
        .messages_handler(|rx: DispatcherHandlerRx<AutoSend<Bot>, Message>| {
            UnboundedReceiverStream::new(rx).for_each_concurrent(None, |cx| async move {
                message_handler(cx).await.log_on_error().await;
            })
        })
        .inline_queries_handler(|rx: DispatcherHandlerRx<AutoSend<Bot>, InlineQuery>| {
            UnboundedReceiverStream::new(rx).for_each_concurrent(None, |cx| async move {
                inline_queries_handler(cx).await.log_on_error().await;
            })
        })
        .dispatch()
        .await;
}
