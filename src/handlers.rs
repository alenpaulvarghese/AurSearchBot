use super::request::{cached_search, AurResponse, Utils};

use std::{error::Error, sync::Arc};

use log::info;
use teloxide::types::{
    InlineKeyboardButton, InlineKeyboardMarkup, InlineQuery, InlineQueryResult,
    InlineQueryResultArticle, InputMessageContent, InputMessageContentText, Message, ParseMode,
};
use teloxide::{prelude::*, utils::command::BotCommand};
use tokio::join;

#[derive(BotCommand)]
#[command(rename = "lowercase", description = "These commands are supported:")]
enum Command {
    #[command(description = "check if I'm alive.")]
    Start,
    #[command(description = "display this text.")]
    Help,
    #[command(description = "search a package.")]
    Search(String),
}

pub async fn inline_queries_handler(
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
    info!(
        "Someone queried about \"{}\", current offset: {}",
        &cx.update.query, &offset
    );
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
    if inline_result.is_empty() {
        inline_result.push(InlineQueryResult::Article(InlineQueryResultArticle::new(
            "1",
            "No result found",
            InputMessageContent::Text(InputMessageContentText::new("No package has been found")),
        )))
    }
    let mut req_builder = cx
        .requester
        .answer_inline_query(cx.update.id, inline_result);
    if offset != 0 {
        req_builder = req_builder.next_offset(offset.to_string());
    }
    req_builder.await?;

    Ok(())
}

pub async fn callback_handler(
    cx: UpdateWithCx<AutoSend<Bot>, CallbackQuery>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let query = cx.update.data;
    match query {
        Some(data) if data == "about" => {
            let message = cx.update.message.unwrap();
            let _d = join!(
                cx.requester.answer_callback_query(cx.update.id).text(""),
                cx.requester.delete_message(message.chat_id(), message.id)
            );

            cx.requester
                .send_message(message.chat_id(), "This project is open ❤️ source")
                .reply_markup(
                    InlineKeyboardMarkup::new([[
                        InlineKeyboardButton::url(
                            "👨🏻‍🦯 Source".to_string(),
                            "https://gitlab.com/alenpaul2001/aursearchbot".to_string(),
                        ),
                        InlineKeyboardButton::url(
                            "❓ Bug Report".to_string(),
                            "https://gitlab.com/alenpaul2001/aursearchbot/-/issues".to_string(),
                        ),
                    ]])
                    .append_row([InlineKeyboardButton::url(
                        "📕 Support".to_string(),
                        "https://t.me/bytessupport".to_string(),
                    )]),
                )
                .await?;
        }
        Some(_) | None => {}
    }
    Ok(())
}

pub async fn message_handler(
    cx: UpdateWithCx<AutoSend<Bot>, Message>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let text = cx.update.text();
    if let None = text {
        return Ok(());
    }
    match Command::parse(text.unwrap(), "AurSearchBot") {
        Ok(command) => {
            match command {
                Command::Start => {
                    cx.answer("Hi 👋, I can search packages on Arch User Repository, Inspired from @FDroidSearchBot").reply_markup(
                        InlineKeyboardMarkup::new(
                            [[
                                InlineKeyboardButton::switch_inline_query_current_chat("Search in inline mode".to_string(),"".to_string()),
                            InlineKeyboardButton::callback("❓ About".to_string(), "about".to_string())]]
                        )
                    ).await?;
                }
                Command::Help => {
                    cx.answer(Command::descriptions()).await?;
                }
                Command::Search(string) => {
                    if string.is_empty() {
                        cx.reply_to("Please provide a search phrase").await?;
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
