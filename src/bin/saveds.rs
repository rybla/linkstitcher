use anyhow::Result;
use chrono::Days;
use dotenvy::dotenv;
use linkstitcher::{
    Env, config, embellish_preview, get_recent_saved_previews, models::Preview, utility,
};
use std::fs;

const FEED_FILENAME: &str = "saveds.feed.xml";
const FEED_TITLE: &str = "linkstitcher/saveds";
const FEED_DESCRIPTION: &str = "The linkstitcher feed for saved URLs.";
const RECENCY_CUTOFF_DAYS: Days = Days::new(7);

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();
    dotenv()?;

    log::trace!("saveds::main");

    let mut env = Env::new()?;

    // fetch previews
    let mut previews = vec![];
    let content = fs::read_to_string(config::SAVED_URLS_FILEPATH.as_str())?;
    let saved_urls = content
        .split("\n")
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>();

    for url in saved_urls {
        if !utility::db::is_url_known(&mut env.db_conn, url)? {
            let mut preview = Preview::from_url(url.to_owned());
            preview.saved = true;
            previews.push(preview);
        }
    }

    // embellish previews
    for preview in previews.iter_mut() {
        if let Err(e) = embellish_preview(&mut env, preview).await {
            log::error!("Error during embellish_preview: {e}");
        }
    }

    // insert previews into database
    for preview in &previews {
        if let Err(e) = utility::db::insert_preview(&mut env.db_conn, preview) {
            log::warn!("Error during insert_preview: {e}");
        }
    }

    // write local RSS channel
    {
        let previews = get_recent_saved_previews(&mut env.db_conn, RECENCY_CUTOFF_DAYS)?;
        utility::rss::write_rss_channel(
            &[config::FEEDS_DIRPATH, FEED_FILENAME].join("/"),
            utility::rss::create_rss_channel(FEED_TITLE, FEED_DESCRIPTION, previews),
        )?;
    }

    // clear urls
    fs::write(config::SAVED_URLS_FILEPATH.as_str(), String::new())?;

    Ok(())
}
