use std::sync::Arc;
use std::time::Duration;

use chrono::NaiveDateTime;
use regex::{Captures, Regex};

use reqwest::Client;
use retainer::{entry::CacheEntryReadGuard, Cache};
use serde::{Deserialize, Deserializer};

pub struct Utils {
    pub cache: Arc<Cache<String, AurResponse>>,
    pub client: Client,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(tag = "type")]
pub enum AurResponse {
    #[serde(rename = "error")]
    Error { error: String },
    #[serde(rename = "search")]
    Result {
        resultcount: usize,
        results: Vec<Packages>,
    },
}

#[derive(Deserialize, Debug, Default, Clone)]
#[serde(rename_all = "PascalCase", default)]
pub struct Packages {
    #[serde(rename = "ID")]
    pub id: u64,
    pub name: String,
    pub version: String,
    #[serde(deserialize_with = "null_to_none")]
    pub description: String,
    pub popularity: f32,
    pub num_votes: u32,
    #[serde(deserialize_with = "null_to_none")]
    pub maintainer: String,
    #[serde(rename = "URL", deserialize_with = "null_to_none")]
    pub package_url: String,
    pub package_base: String,
    #[serde(deserialize_with = "posix_to_datefrmt")]
    pub first_submitted: String,
    #[serde(deserialize_with = "posix_to_datefrmt")]
    pub last_modified: String,
}

// convert null type json objects to literal None
fn null_to_none<'de, D>(de: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    let s: String = Deserialize::deserialize(de).unwrap_or(String::from("None"));
    let regex = Regex::new(r"(<|>|&)").unwrap();
    // properly escape special characters.
    // https://docs.rs/teloxide/0.5.3/teloxide/types/enum.ParseMode.html#html-style
    let result = regex.replace_all(&s, |cap: &Captures| match &cap[0] {
        ">" => "&gt;",
        "<" => "&lt;",
        "&" => "&amp;",
        _ => panic!("We should never get here"),
    });
    Ok(result.to_string())
}

// convert posix string to date format
fn posix_to_datefrmt<'de, D>(de: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    let timestamp: i64 = Deserialize::deserialize(de)?;
    let naive = NaiveDateTime::from_timestamp(timestamp, 0);
    Ok(naive.format("%Y-%m-%d %H:%M").to_string())
}

impl Packages {
    pub fn git(&self) -> String {
        format!("https://aur.archlinux.org/{}.git", self.package_base)
    }

    pub fn pretty(&self) -> String {
        format!(
            "📦 <b>{}</b>\n\n\
            ℹ️{}\n\n\
            🔗<a href='{}'>Git</a> | \
            <a href='{}'>Source</a>\n\
            - Maintainer: <code>{}</code>\n\
            - Votes: <code>{}</code>\n\
            - Version: <code>{}</code>\n\
            - Popularity: <code>{}</code>\n\
            - Last Updated: <code>{}</code>\n\
            - First Submitted: <code>{}</code>
            ",
            self.name,
            &self.description,
            self.git(),
            &self.package_url,
            &self.maintainer,
            self.num_votes,
            self.version,
            self.popularity,
            &self.last_modified,
            &self.first_submitted,
        )
    }
}

pub async fn search(client: &Client, package: &str) -> AurResponse {
    let params = [
        ("v", "5"),
        ("type", "search"),
        ("by", "name"),
        ("arg", package),
    ];
    let res = client
        .get("https://aur.archlinux.org/rpc.php/rpc/")
        .query(&params)
        .send()
        .await
        .unwrap();
    res.json::<AurResponse>().await.unwrap()
}

pub async fn cached_search<'a>(
    utils: &'a Utils,
    query: &String,
) -> CacheEntryReadGuard<'a, AurResponse> {
    // check for cached entry
    if let Some(cache) = utils.cache.get(query).await {
        cache
    } else {
        // if entry not found search the package in AUR
        let mut response = search(&utils.client, query).await;
        if let AurResponse::Result { results, .. } = &mut response {
            // sort result based on popularity
            results.sort_by(|a, b| b.popularity.partial_cmp(&a.popularity).unwrap());
        }
        // add the result to cache
        utils
            .cache
            .insert(query.clone(), response, Duration::from_secs(60))
            .await;
        utils.cache.get(query).await.unwrap()
    }
}
