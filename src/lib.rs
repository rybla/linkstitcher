pub mod config;
pub mod models;
pub mod schema;
pub mod utility;

pub use models::*;
pub use schema::*;

use anyhow::{Result, anyhow};
use diesel::{dsl::exists, insert_into, prelude::*, select};
use octocrab::Octocrab;
use readability_js::Readability;
use scraper::Html;
use std::{
    fs::{self, File},
    io::{BufWriter, Write},
};

pub fn establish_connection() -> SqliteConnection {
    SqliteConnection::establish(&config::DATABASE_URL)
        .unwrap_or_else(|_| panic!("Error connecting to {}", config::DATABASE_URL.as_str()))
}

pub fn set_preview(conn: &mut SqliteConnection, preview: Preview) -> Result<Option<Preview>> {
    Ok(insert_into(previews::table)
        .values(&preview)
        .returning(Preview::as_returning())
        .get_result(conn)
        .optional()?)
}

pub fn get_preview(conn: &mut SqliteConnection, url: String) -> Result<Option<Preview>> {
    use previews::dsl;

    Ok(dsl::previews
        .find(url)
        .select(Preview::as_select())
        .first(conn)
        .optional()?)
}

pub fn exists_preview(conn: &mut SqliteConnection, url: &str) -> Result<bool> {
    use previews::dsl;

    Ok(select(exists(dsl::previews.find(url))).get_result(conn)?)
}

/// Gets all previews with an `added_date` within the last `days`.
pub fn get_recent_previews(
    conn: &mut SqliteConnection,
    days: chrono::Days,
) -> Result<Vec<Preview>> {
    use previews::dsl;

    let then = chrono::Utc::now()
        .date_naive()
        .checked_sub_days(days)
        .unwrap();

    Ok(dsl::previews
        .filter(dsl::added_date.gt(then))
        .select(Preview::as_select())
        .load(conn)?)
}

pub struct Env<'a> {
    pub client: &'a reqwest::Client,
    pub conn: &'a mut SqliteConnection,
    pub readability: &'a Readability,
    pub octocrab: &'a Octocrab,
}

pub async fn fill_initial_preview(env: &mut Env<'_>, preview: &mut Preview) -> Result<()> {
    if exists_preview(env.conn, &preview.url)? {
        return Ok(());
    }

    log::info!["creating new preview for {}", &preview.url];

    let mut content: Option<String> = None;

    if let Some(arxiv_id) = utility::arxiv::get_id_from_url(&preview.url) {
        if let Ok(article) = utility::arxiv::fetch_by_id(arxiv_id).await {
            preview.title = Some(article.title);
            // NOTE: could use DateTime::parse_from_rfc3339
            preview.published_date = Some(article.published);
            if preview.source.is_none() {
                preview.source = Some("ArXiv".to_owned())
            }
            preview.tags = Some(article.category_names.join(", "));
            preview.summary = Some(article.summary.clone());
        } else {
            log::error!["failed to fetch ArXiv article: {}", preview.url];
        }
    } else if preview.url.starts_with("https://x.com/") {
        if let Ok(post) = utility::x::fetch_post(&preview.url).await {
            let html = Html::parse_fragment(&post.html);
            let mut content = String::new();
            for text in html.root_element().text() {
                content.push_str(&format!(" {text}"));
            }
            preview.summary = Some(content.chars().take(config::MAX_CHARS_SUMMARY).collect());
        } else {
            log::error!["failed to fetch X post: {}", preview.url];
        }
    } else if preview.url.contains("github.com") {
        if let Ok(info) = utility::github::fetch_repo_info(env.octocrab, &preview.url).await {
            preview.summary = info
                .readme
                .map(|s| s.chars().take(config::MAX_CHARS_SUMMARY).collect());
        } else {
            log::error!["failed to fetch GitHub repo info: {}", preview.url];
        }
    } else {
        // fetch content at URL
        let response = env.client.get(&preview.url).send().await?;
        let headers = response.headers();

        let content_type = match headers.get("content-type") {
            None => {
                return Result::Err(anyhow!(
                    "I failed to get the content type, since the response does not have a header for content-type: {response:?}"
                ));
            }
            Some(content_type) => {
                let bytes = content_type.as_bytes();
                let str = String::from_utf8_lossy(bytes);
                str.to_string()
            }
        };

        // extract content
        #[allow(clippy::single_match)]
        match content_type.as_str() {
            "text/pdf" => {
                let mut file = tempfile::Builder::new().suffix(".pdf").tempfile()?;
                let bytes = response.bytes().await?;
                file.write_all(&bytes)?;
                let text = pdf_extract::extract_text(file.path().to_str().ok_or(anyhow!["TODO"])?)?;

                content = Some(text);
            }
            "text/html" => {
                let html = response.text().await?;
                match env.readability.parse_with_url(&html, &preview.url) {
                    Err(e) => {
                        log::warn!["failed to use Readability to parse with url: {e}"];
                        preview.title = Some(preview.url.clone());
                    }
                    Ok(article) => {
                        // let content = &article.text_content;
                        // let byline = &article.byline;
                        // let published_date = &article.published_time;
                        // let title = &article.title;

                        preview.title = Some(article.title.clone());
                        if let Some(pub_date) = article.published_time {
                            preview.published_date = Some(pub_date);
                        }

                        content = Some(article.content.clone());
                    }
                }
            }
            // TODO: handle other types of content
            _ => {}
        }

        if preview.summary.is_none()
            && let Some(content) = &content
        {
            preview.summary = Some(content.chars().take(config::MAX_CHARS_SUMMARY).collect());
        }
    }

    Ok(())
}

pub async fn fill_detailed_preview(env: &mut Env<'_>, preview: &mut Preview) -> Result<()> {
    fill_initial_preview(env, preview).await?;
    todo!()
}

// -----------------------------------------------------------------------------

pub fn create_rss_channel(previews: Vec<Preview>) -> rss::Channel {
    rss::ChannelBuilder::default()
        .title("rybl/linkstitcher")
        .image(rss::Image {
            url: "https://www.rybl.net/favicon.ico".to_owned(),
            title: "rybl/linkstitcher".to_owned(),
            link: config::REPOSITORY_URL.to_string(),
            width: None,
            height: None,
            description: None,
        })
        .link(config::REPOSITORY_URL.to_string())
        .description("This is the linkstitcher RSS feed for https://github.com/rybla")
        .items(
            previews
                .into_iter()
                .map(rss::Item::from)
                .collect::<Vec<_>>(),
        )
        .build()
}

pub fn write_rss_channel(channel: rss::Channel) -> Result<()> {
    let file = File::create(config::FEED_FILEPATH)?;
    let writer = BufWriter::new(file);
    channel.pretty_write_to(writer, b' ', 4)?;
    Ok(())
}

pub async fn fetch_rss_channel(client: &reqwest::Client, url: &str) -> Result<rss::Channel> {
    let content = client.get(url).send().await?;
    let bytes = content.bytes().await?;
    let channel = rss::Channel::read_from(&bytes[..])?;
    Ok(channel)
}

pub fn fetch_urls() -> Result<Vec<String>> {
    let content = fs::read_to_string(config::URLS_FILEPATH.as_str())?;

    Ok(content
        .split('\n')
        .filter(|s| !s.is_empty())
        .map(|s| s.to_owned())
        .collect())
}

// TODO: use this
pub fn clear_urls() -> Result<()> {
    fs::write(config::URLS_FILEPATH.as_str(), "")?;
    Ok(())
}

pub fn fetch_rss_feed_urls() -> Result<Vec<String>> {
    let content = fs::read_to_string(config::RSS_FEED_URLS_FILEPATH.as_str())?;
    Ok(content
        .split('\n')
        .filter(|s| !s.is_empty())
        .map(|s| s.to_owned())
        .collect())
}
