use std::sync::Arc;
use std::time::Duration;

use chrono::NaiveDateTime;
use regex::Regex;

use reqwest::Client;
use retainer::{entry::CacheEntryReadGuard, Cache};
use serde::{Deserialize, Deserializer};

use lazy_static::lazy_static;

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

pub enum Search {
    Package(String),
    Maintainer(String),
}

impl Search {
    pub fn from(query: &str) -> Self {
        if query.starts_with("!m ") {
            Search::Maintainer(query.replace("!m ", ""))
        } else {
            Search::Package(query.to_string())
        }
    }
}

impl std::ops::Deref for Search {
    type Target = String;
    fn deref(&self) -> &Self::Target {
        match &self {
            Search::Package(x) => x,
            Search::Maintainer(x) => x,
        }
    }
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

// convert null type json objects to literal None and properly escape special characters.
// https://docs.rs/teloxide/0.5.3/teloxide/types/enum.ParseMode.html#html-style
fn null_to_none<'de, D>(de: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    lazy_static! {
        static ref REGEX: Regex = Regex::new(r"[<>&]").unwrap();
    }
    let string: String = Deserialize::deserialize(de).unwrap_or(String::from("None"));
    // https://lise-henry.github.io/articles/optimising_strings.html
    let first = REGEX.find(&string);
    if let Some(first) = first {
        let first = first.start();
        let mut output = String::from(&string[0..first]);
        output.reserve(string.len() - first);
        let rest = string[first..].chars();
        for c in rest {
            match c {
                '<' => output.push_str("&lt;"),
                '>' => output.push_str("&gt;"),
                '&' => output.push_str("&amp;"),
                _ => output.push(c),
            }
        }
        return Ok(output);
    }
    Ok(string)
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

pub async fn search(client: &Client, query: &Search) -> AurResponse {
    let get_by = || match query {
        &Search::Maintainer(_) => ("by", "maintainer"),
        &Search::Package(_) => ("by", "name"),
    };
    let params = [("v", "5"), ("type", "search"), get_by(), ("arg", query)];
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
    query: Search,
) -> CacheEntryReadGuard<'a, AurResponse> {
    // check for cached entry
    if let Some(cache) = utils.cache.get(&query).await {
        cache
    } else {
        // if entry not found search the package in AUR
        let mut response = search(&utils.client, &query).await;
        if let AurResponse::Result { results, .. } = &mut response {
            // sort result based on popularity
            results.sort_by(|a, b| b.popularity.partial_cmp(&a.popularity).unwrap());
        }
        // add the result to cache
        utils
            .cache
            .insert(query.clone(), response, Duration::from_secs(60))
            .await;
        utils.cache.get(&query).await.unwrap()
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn test_request_functions() {
        use crate::request::cached_search;
        use crate::request::{AurResponse, Search};
        use crate::{Cache, Client, Utils};
        use std::sync::Arc;

        let cache = Arc::new(Cache::new());
        let utils = Utils {
            cache: Arc::clone(&cache),
            client: Client::new(),
        };
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let result = runtime.block_on(cached_search(&utils, Search::from("paru")));
        assert!(
            matches!(*result, AurResponse::Result { .. },),
            "Search failed with a reponse of error variant"
        );
        if let AurResponse::Result {
            results,
            resultcount,
        } = &*result
        {
            assert_ne!(
                *resultcount, 0,
                "Number of packages returned from search is zero",
            );

            assert_eq!(results[0].name, "paru", "The packages sorting failed");
            assert_eq!(
                results[0].git(),
                "https://aur.archlinux.org/paru.git",
                "Invalid git url found for package"
            );
        }
        let result = runtime.block_on(utils.cache.get(&String::from("paru")));
        assert_ne!(matches!(result, None), true, "Couldn't find cache hit");
        runtime.shutdown_background();
    }
}
