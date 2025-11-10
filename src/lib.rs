pub mod config;
pub mod models;
pub mod schema;
pub mod utility;

pub use models::*;
pub use schema::*;

use anyhow::{Result, anyhow};
use diesel::prelude::*;
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

pub fn insert_or_update_preview(conn: &mut SqliteConnection, preview: &Preview) -> Result<()> {
    if exists_preview(conn, &preview.url)? {
        update_preview(conn, preview)?;
    } else {
        insert_preview(conn, preview)?;
    }

    Ok(())
}

pub fn insert_preview(conn: &mut SqliteConnection, preview: &Preview) -> Result<()> {
    use crate::previews::dsl;

    diesel::insert_into(dsl::previews)
        .values(preview)
        .execute(conn)?;
    Ok(())
}

pub fn update_preview(conn: &mut SqliteConnection, preview: &Preview) -> Result<()> {
    use crate::previews::dsl;

    diesel::update(dsl::previews.find(&preview.url))
        .set((
            dsl::source.eq(&preview.source),
            dsl::title.eq(&preview.title),
            dsl::published_date.eq(&preview.published_date),
            dsl::tags.eq(&preview.tags),
            dsl::summary.eq(&preview.summary),
            dsl::bookmarked.eq(&preview.bookmarked),
        ))
        .execute(conn)?;
    Ok(())
}

pub fn get_preview(conn: &mut SqliteConnection, url: String) -> Result<Option<Preview>> {
    use crate::previews::dsl::previews;

    Ok(previews
        .find(url)
        .select(Preview::as_select())
        .first(conn)
        .optional()?)
}
pub fn exists_preview(conn: &mut SqliteConnection, url: &str) -> Result<bool> {
    use crate::previews::dsl::previews;

    Ok(previews
        .find(url)
        .first::<Preview>(conn)
        .optional()?
        .is_some())
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

pub struct Env {
    pub client: reqwest::Client,
    pub conn: SqliteConnection,
    pub readability: Readability,
    pub octocrab: Octocrab,
}

impl Env {
    pub fn new() -> Result<Self> {
        let conn = establish_connection();

        let readability = Readability::new()?;

        let octocrab = Octocrab::builder()
            .personal_token(config::GITHUB_PERSONAL_ACCESS_TOKEN.as_str())
            .build()
            .unwrap();

        let client = reqwest::Client::new();

        Ok(Env {
            conn,
            readability,
            octocrab,
            client,
        })
    }
}

pub async fn fill_initial_preview(env: &mut Env, preview: &mut Preview) -> Result<Option<String>> {
    log::info!["Filling initial preview: {}", &preview.url];

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
            content = Some(article.summary.clone());
        } else {
            log::error!["failed to fetch ArXiv article: {}", preview.url];
        }
    } else if preview.url.starts_with("https://x.com/") {
        if let Ok(post) = utility::x::fetch_post(&preview.url).await {
            let html = Html::parse_fragment(&post.html);
            let mut text = String::new();
            for s in html.root_element().text() {
                text.push_str(&format!(" {s}"));
            }
            preview.summary = Some(text.chars().take(config::MAX_CHARS_SUMMARY).collect());
            content = Some(text.clone());
        } else {
            log::error!["failed to fetch X post: {}", preview.url];
        }
    } else if preview.url.contains("github.com") {
        if let Ok(info) = utility::github::fetch_repo_info(&env.octocrab, &preview.url).await {
            content = info.readme.clone();
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
    }

    if preview.summary.is_none()
        && let Some(content) = &content
    {
        preview.summary = Some(content.chars().take(config::MAX_CHARS_SUMMARY).collect());
    }

    Ok(content)
}

pub async fn fill_bookmarked_preview(env: &mut Env, preview: &mut Preview) -> Result<()> {
    log::info!("Filling bookmarked preview: {}", preview.url);

    if let Some(content) = fill_initial_preview(env, preview).await? {
        log::info!("Creating tags for preview: {}", preview.url);

        let output = utility::ai::gemini_cli(&format!(
            "Write a comma-separated list of tags that categorize the following written content. Respond ONLY with the comma-separated list.\n\n{content}"
        ))?;

        let tags = [
            preview.tags().unwrap_or_default(),
            output
                .trim()
                .split(",")
                .map(|tag| tag.trim())
                .filter(|tag| !tag.is_empty())
                .collect::<Vec<_>>(),
        ]
        .concat();

        preview.tags = Some(tags.join(","));
        // fs::write(config::BOOKMARKED_URLS_WITH_TAGS_FILEPATH, contents)
    } else {
        log::warn!(
            "I failed to create tags for this preview since no content was extracted for it: {}",
            preview.url
        );
    }

    preview.bookmarked = true;

    Ok(())
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

pub fn get_saved_urls() -> Result<Vec<String>> {
    let content = fs::read_to_string(config::SAVED_URLS_FILEPATH.as_str())?;

    Ok(content
        .split('\n')
        .filter(|s| !s.is_empty())
        .map(|s| s.to_owned())
        .collect())
}

pub fn clear_saved_urls() -> Result<()> {
    fs::write(config::SAVED_URLS_FILEPATH.as_str(), "")?;
    Ok(())
}

pub fn get_bookmarked_urls() -> Result<Vec<String>> {
    let saved_urls = std::fs::read_to_string(config::BOOKMARKED_URLS_FILEPATH.as_str())?;
    let saved_urls = saved_urls
        .split("\n")
        .filter(|s| !s.is_empty())
        .map(|s| s.to_owned())
        .collect::<Vec<_>>();
    Ok(saved_urls)
}

pub fn clear_bookmarked_urls() -> Result<()> {
    std::fs::write(config::BOOKMARKED_URLS_FILEPATH.as_str(), "")?;
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
