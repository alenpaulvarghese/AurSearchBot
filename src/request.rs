use chrono::NaiveDateTime;
use reqwest::Client;
use serde::{Deserialize, Deserializer};

#[derive(Deserialize, Debug)]
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

#[derive(Deserialize, Debug, Default)]
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
    Ok(s)
}

fn posix_to_datefrmt<'de, D>(de: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    let timestamp: i64 = Deserialize::deserialize(de)?;
    // let timestamp = s.parse::<i64>().unwrap();
    let naive = NaiveDateTime::from_timestamp(timestamp, 0);
    Ok(naive.format("%Y-%m-%d %H:%M").to_string())
}

impl Packages {
    pub fn git(&self) -> String {
        format!("https://aur.archlinux.org/{}.git", self.package_base)
    }

    pub fn pretty(&self) -> String {
        format!(
            "Package Details: <b>{} <i>{}</i></b>\n\n\
            <b>Git Clone URL:</b> {}\n\n\
            <b>Description</b>: <i>{}</i>\n\
            <b>Upstream URL</b>: {}\n\
            <b>Maintainer</b>: <i>{}</i>\n\
            <b>Votes</b>: <i>{}</i>\n\
            <b>Popularity</b>: <i>{}</i>\n\
            <b>First Submitted</b>: <i>{}</i>\n\
            <b>Last Updated</b>: <i>{}</i>
            ",
            self.name,
            self.version,
            self.git(),
            &self.description,
            &self.package_url,
            &self.maintainer,
            self.num_votes,
            self.popularity,
            &self.first_submitted,
            &self.last_modified,
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
    let text = res.text().await.unwrap();
    serde_json::from_str(&text).unwrap()
}
