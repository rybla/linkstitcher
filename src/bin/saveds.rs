use anyhow::Result;
use linkstitcher::{Env, config, embellish_preview, models::Preview, utility};
use std::fs;

#[tokio::main]
async fn main() -> Result<()> {
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
            let preview = Preview::from_url(url.to_owned());
            previews.push(preview);
        }
    }

    // embellish previews
    for preview in previews.iter_mut() {
        embellish_preview(&mut env, preview).await?;
    }

    // insert previews into database
    for preview in &previews {
        utility::db::insert_preview(&mut env.db_conn, preview)?;
    }

    // write local RSS channel
    utility::rss::write_rss_channel(
        &[config::FEEDS_DIRPATH, "saveds.feed.xml"].join("/"),
        utility::rss::create_rss_channel(
            "linkstitcher/saveds",
            "The linkstitcher feed for saved URLs.",
            previews,
        ),
    )?;

    Ok(())
}
