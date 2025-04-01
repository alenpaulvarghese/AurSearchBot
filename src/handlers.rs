use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use log::info;
use teloxide::sugar::request::RequestLinkPreviewExt;
use teloxide::types::{
    ChatId, InlineKeyboardButton, InlineKeyboardMarkup, InlineQueryResult, InlineQueryResultArticle,
    InlineQueryResultsButton, InlineQueryResultsButtonKind, InputFile, InputMessageContent, InputMessageContentText,
    ParseMode,
};
use teloxide::{RequestError, prelude::*, utils::command::BotCommands};

use crate::request::{AurResponse, Search, Utils, cached_search};

#[derive(BotCommands)]
#[command(rename_rule = "lowercase", description = "These commands are supported:")]
enum Command {
    #[command(description = "check if I'm alive.")]
    Start,
    #[command(description = "display this text.")]
    Help,
    #[command(description = "hide")]
    Debug,
}

pub async fn inline_queries_handler(bot: Bot, update: InlineQuery, utils: Arc<Utils>) -> Result<(), RequestError> {
    // check if the query is empty or contain certain characters
    match update.query.as_str() {
        "" | "!" | "!m" | "!m " => {
            bot.answer_inline_query(update.id, [])
                .button(InlineQueryResultsButton {
                    text: "Type to search packages on AUR".to_string(),
                    kind: InlineQueryResultsButtonKind::StartParameter("start".to_string()),
                })
                .await?;
            return respond(());
        }
        _ => {}
    }
    let mut inline_result: Vec<InlineQueryResult> = Vec::new();
    let mut offset = update.offset.parse::<usize>().unwrap_or_default();
    let instant = Instant::now();
    let aur_response = cached_search(&utils, Search::from(&update.query)).await;
    match &*aur_response {
        AurResponse::Result { total, results } => {
            info!(
                "Query: \"{}\", total result: {}, current offset: {}, took: {}ms",
                update.query,
                total,
                offset,
                instant.elapsed().as_millis()
            );
            results.iter().skip(offset).take(50).for_each(|package| {
                inline_result.push(InlineQueryResult::Article(
                    InlineQueryResultArticle::new(
                        package.id.to_string(),
                        &package.name,
                        InputMessageContent::Text(
                            InputMessageContentText::new(&package.pretty()).parse_mode(ParseMode::Html),
                        ),
                    )
                    .description(&package.description),
                ))
            });
            // increase the offset by 50 after every scroll down
            // if the current offset + 50 is lesser than the total
            // length of the result the offset should be set to 0
            offset = if offset + 50 < results.len() { offset + 50 } else { 0 };
        }
        AurResponse::Error { error } => {
            info!("Query: \"{}\", error: {}", update.query, error);
            inline_result.push(InlineQueryResult::Article(InlineQueryResultArticle::new(
                "1",
                error,
                InputMessageContent::Text(InputMessageContentText::new("Error occurred while searching AUR")),
            )))
        }
    };
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

pub async fn message_handler(bot: Bot, message: Message) -> Result<(), RequestError> {
    let text = message.text();
    if text.is_none() {
        return respond(());
    }
    if let Ok(command) = Command::parse(text.unwrap(), "AurSearchBot") {
        match command {
            Command::Start => {
                bot.send_message(
                    message.chat.id,
                    "This bot searches Packages in <a href='https://aur.archlinux.org/'>\
                     AUR repository</a>, works only in inline mode \
                Inspired from @FDroidSearchBot\n\nCurrently supported search patterns:\n\
                - <code>Packages</code>, search directly\n- <code>Maintainer</code>, search with <code>!m</code>\n\n\
                <a href='https://github.com/alenpaulvarghese/aursearchbot'>Source Code</a> | \
                <a href='https://t.me/bytesio'>Developer</a> | <a href='https://t.me/bytessupport'>Support Chat</a>",
                )
                .reply_markup(InlineKeyboardMarkup::new([[
                    InlineKeyboardButton::switch_inline_query_current_chat("Search Packages", String::new()),
                    InlineKeyboardButton::switch_inline_query_current_chat("Search Package by Maintainers", "!m "),
                ]]))
                .parse_mode(ParseMode::Html)
                .disable_link_preview(false)
                .await?;
            }
            Command::Help => {
                bot.send_message(message.chat.id, Command::descriptions().to_string())
                    .await?;
            }
            Command::Debug => {
                let su_user_id = std::env::var("SU_USER").unwrap_or_default().parse::<i64>();
                if su_user_id.is_err() {
                    return respond(());
                }
                let su_user = ChatId(su_user_id.unwrap());
                if su_user == message.chat.id {
                    let file_name = PathBuf::from("debug.log");
                    if file_name.exists() {
                        bot.send_document(message.chat.id, InputFile::file(file_name)).await?;
                    } else {
                        bot.send_message(message.chat.id, "No log files found").await?;
                    }
                }
            }
        };
    };
    respond(())
}
