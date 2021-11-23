use std::sync::Arc;
use std::time::Duration;

use chrono::NaiveDateTime;
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
        resultcount: u32,
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

fn null_to_none<'de, D>(de: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    let s: String = Deserialize::deserialize(de).unwrap_or(String::from("None"));
    // temporary fix for parsing error
    if s.contains("<=>") {
        return Ok(s.replace("<=>", ""));
    }
    Ok(s)
}

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
