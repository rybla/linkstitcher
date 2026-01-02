use anyhow::Result;
use dotenvy::dotenv;
use linkstitcher::{Env, bookmark_preview, config, embellish_preview, models::Preview, utility};
use std::fs;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();
    dotenv()?;

    log::trace!("bookmarks::main");

    let mut env = Env::new()?;

    // fetch previews
    let content = fs::read_to_string(config::BOOKMARKED_URLS_FILEPATH.as_str())?;
    let bookmark_urls = content
        .split("\n")
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>();

    for url in bookmark_urls {
        let existing_preview = utility::db::get_preview(&mut env.db_conn, url.to_owned())?;
        let preview = existing_preview.unwrap_or_else(|| Preview::from_url(url.to_owned()));
        let mut preview = if preview.embellished {
            preview
        } else {
            let mut preview = preview;
            if let Err(e) = embellish_preview(&mut env, &mut preview).await {
                log::error!("Error during embellish_preview: {e}");
            }
            preview
        };
        bookmark_preview(&mut env, &mut preview).await?;
        utility::db::insert_or_update_preview(&mut env.db_conn, &preview)?;
    }

    // clear urls
    fs::write(config::BOOKMARKED_URLS_FILEPATH.as_str(), String::new())?;

    Ok(())
}
