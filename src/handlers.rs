use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use log::info;
use teloxide::types::{
    InlineKeyboardButton, InlineKeyboardMarkup, InlineQueryResult, InlineQueryResultArticle,
    InputFile, InputMessageContent, InputMessageContentText, ParseMode,
};
use teloxide::{prelude2::*, utils::command::BotCommand, RequestError};

use crate::request::{cached_search, AurResponse, Search, Utils};

#[derive(BotCommand)]
#[command(rename = "lowercase", description = "These commands are supported:")]
enum Command {
    #[command(description = "check if I'm alive.")]
    Start,
    #[command(description = "display this text.")]
    Help,
    #[command(description = "off")]
    Debug,
}

pub async fn inline_queries_handler(
    bot: AutoSend<Bot>,
    update: InlineQuery,
    utils: Arc<Utils>,
) -> Result<(), RequestError> {
    // check if the query is empty or contain certain characters
    match update.query.as_str() {
        "" | "!" | "!m" | "!m " => {
            bot.answer_inline_query(update.id, [])
                .switch_pm_text("Type to search packages on AUR")
                .switch_pm_parameter("start")
                .await?;
            return Ok(());
        }
        _ => {}
    }
    let mut inline_result: Vec<InlineQueryResult> = Vec::new();
    let mut offset = update.offset.parse::<usize>().unwrap_or_default();
    let instant = Instant::now();
    let aur_response = cached_search(&utils, Search::from(&update.query)).await;
    if let AurResponse::Result { results, total } = &*aur_response {
        info!(
            "Query: \"{}\", total result: {}, current offset: {}, took: {}ms",
            update.query,
            total,
            offset,
            instant.elapsed().as_millis()
        );
        let mut end = offset + 50;
        if end > *total {
            end = *total
        }
        for items in &results[offset..end] {
            inline_result.push(InlineQueryResult::Article(
                InlineQueryResultArticle::new(
                    items.id.to_string(),
                    &items.name,
                    InputMessageContent::Text(
                        InputMessageContentText::new(&items.pretty())
                            .parse_mode(ParseMode::Html)
                            .disable_web_page_preview(true),
                    ),
                )
                .description(&items.description),
            ));
        }
        // increase the offset by 50 after every scroll down
        // if the current offset + 50 is lesser than the total
        // length of the result the offset should be set to 0
        offset = if offset + 50 < results.len() {
            offset + 50
        } else {
            0
        };
    } else if let AurResponse::Error { error } = &*aur_response {
        info!("Query: \"{}\", error: {}", update.query, error);
        inline_result.push(InlineQueryResult::Article(InlineQueryResultArticle::new(
            "1",
            error,
            InputMessageContent::Text(InputMessageContentText::new(
                "Error occurred while searching AUR",
            )),
        )))
    }
    if inline_result.is_empty() {
        inline_result.push(InlineQueryResult::Article(InlineQueryResultArticle::new(
            "1",
            "No result found",
            InputMessageContent::Text(InputMessageContentText::new("No package has been found")),
        )))
    }
    let mut req_builder = bot.answer_inline_query(update.id, inline_result);
    if offset != 0 {
        req_builder = req_builder.next_offset(offset.to_string());
    }
    req_builder.await?;

    respond(())
}

pub async fn message_handler(bot: AutoSend<Bot>, message: Message) -> Result<(), RequestError> {
    let text = message.text();
    if text.is_none() {
        return respond(());
    }
    if let Ok(command) = Command::parse(text.unwrap(), "AurSearchBot") {
        match command {
            Command::Start => {
                bot.send_message(
                    message.chat_id(),
                    "This bot searches Packages in <a href='https://aur.archlinux.org/'>\
                AUR repository</a>, works only in inline mode \
                Inspired from @FDroidSearchBot\n\nCurrently supported search patterns:\n\
                - <code>Packages</code>, search directly\n- <code>Maintainer</code>, search with <code>!m</code>\n\n\
                <a href='https://gitlab.com/alenpaul2001/aursearchbot'>Source Code</a> | \
                <a href='https://t.me/bytesio'>Developer</a> | <a href='https://t.me/bytessupport'>Support Chat</a>",
                )
                .reply_markup(InlineKeyboardMarkup::new([
                    [
                    InlineKeyboardButton::switch_inline_query_current_chat(
                        "Search Packages".to_string(),
                        String::new(),
                    ),
                    InlineKeyboardButton::switch_inline_query_current_chat(
                        "Search Package by Maintainers".to_string(),
                        "!m ".to_string(),
                    )
                ]]))
                .parse_mode(ParseMode::Html)
                .disable_web_page_preview(true)
                .await?;
            }
            Command::Help => {
                bot.send_message(message.chat_id(), Command::descriptions())
                    .await?;
            }
            Command::Debug => {
                let file_name = PathBuf::from("debug.log");
                if file_name.exists() {
                    bot.send_document(message.chat_id(), InputFile::file(file_name))
                        .await?;
                } else {
                    bot.send_message(message.chat_id(), "No log files found")
                        .await?;
                }
            }
        };
    };
    respond(())
}
