use crate::models::Preview;
use anyhow::{Result, anyhow};
use diesel::prelude::*;
use std::io::Write;

pub mod config;
pub mod models;
pub mod rss_channel;
pub mod schema;
pub mod utility;

pub struct Env {
    pub client: reqwest::Client,
    pub db_conn: diesel::SqliteConnection,
    pub readability: readability_js::Readability,
    pub octocrab: octocrab::Octocrab,
}

impl Env {
    pub fn new() -> Result<Self> {
        let conn = utility::db::establish_connection();

        let readability = readability_js::Readability::new()?;

        let octocrab = octocrab::Octocrab::builder()
            .personal_token(config::GITHUB_PERSONAL_ACCESS_TOKEN.as_str())
            .build()
            .unwrap();

        let client = reqwest::Client::new();

        Ok(Env {
            db_conn: conn,
            readability,
            octocrab,
            client,
        })
    }
}

/// Embellishes a preview with basic content, inexpensively.
pub async fn embellish_preview(env: &mut Env, preview: &mut Preview) -> Result<Option<String>> {
    log::info!["embellish_preview({})", &preview.url];

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
            let html = scraper::Html::parse_fragment(&post.html);
            let mut text = String::new();
            for s in html.root_element().text() {
                text.push_str(&format!(" {s}"));
            }
            preview.summary = Some(text.chars().take(config::MAX_CHARS_SUMMARY).collect());
            content = Some(text.clone());
        } else {
            log::error!["failed to fetch X post: {}", preview.url];
        }
    } else if preview.url.starts_with("https://github.com") {
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
                let file_path = file
                    .path()
                    .to_str()
                    .ok_or(anyhow!("failed to convert file path to String"))?;
                let text = pdf_extract::extract_text(file_path)?;
                content = Some(text);
            }
            content_type if content_type.starts_with("text/html") => {
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

                        println!("article = {article:#?}");

                        preview.title = Some(article.title.clone());
                        if let Some(pub_date) = article.published_time {
                            preview.published_date = Some(pub_date);
                        }

                        content = Some(article.text_content.clone());
                    }
                }
            }
            // TODO: handle other types of content
            _ => {
                log::warn!("unrecognized content type: {content_type}");
            }
        }
    }

    // if still no summary, use truncated content as summary
    if preview.summary.is_none()
        && let Some(content) = &content
    {
        preview.summary = Some(content.chars().take(config::MAX_CHARS_SUMMARY).collect());
    }

    // if still no summary, use title as summary
    if preview.summary.is_none()
        && let Some(title) = &preview.title
    {
        preview.summary = Some(format!("Title: {title}"));
    }

    // prepend source to summary
    if let Some(summary) = &preview.summary
        && let Some(source) = preview.source.clone()
    {
        preview.summary = Some(format!("Source: {source}\n\n{summary}"));
    }

    Ok(content)
}

/// Requires input preview to already be embellished.
pub async fn bookmark_preview(_env: &mut Env, preview: &mut Preview) -> Result<()> {
    log::info!["bookmark_preview({})", &preview.url];

    // generate tags
    if preview.tags.is_none()
        && let Some(title) = &preview.title
        && let Some(summary) = &preview.summary
    {
        let response = utility::ai::gemini_cli(&format!(
            "Consider the following content:\n\nTitle: {title}\n\nText:\n\n{summary}...\n\nWrite a comma-separated list of categorizational tags for the above content. Respond ONLY with the comma-separated list"
        ))?;
        preview.tags = Some(response);
    }

    preview.bookmarked = true;

    Ok(())
}

pub fn get_recent_saved_previews(
    conn: &mut diesel::SqliteConnection,
    days: chrono::Days,
) -> Result<Vec<Preview>> {
    use schema::previews::dsl;

    let then = chrono::Utc::now()
        .date_naive()
        .checked_sub_days(days)
        .unwrap();

    Ok(dsl::previews
        .filter(dsl::added_date.gt(then))
        .filter(dsl::saved.eq(true))
        .select(Preview::as_select())
        .load(conn)?)
}
