use anyhow::Result;
use dotenvy::dotenv;
use linkstitcher::config::*;
use linkstitcher::*;

#[tokio::main]
pub async fn main() -> Result<()> {
    env_logger::init();
    dotenv().ok();

    let mut env = Env::new()?;

    log::info!["Begin fetching URLs"];

    let mut new_previews: Vec<Preview> = vec![];

    log::info!["fetching new previews from urls"];
    {
        let urls = get_saved_urls()?;
        for url in urls {
            new_previews.push(Preview::from_url(url));
        }
    }

    log::info!["fetching new previews from RSS feeds"];
    {
        let rss_feed_urls = fetch_rss_feed_urls()?;
        for rss_feed_url in rss_feed_urls {
            match fetch_rss_channel(&env.client, &rss_feed_url).await {
                Ok(channel) => {
                    let mut items = channel.items;
                    items.truncate(MAX_RSS_FEED_ITEMS);
                    for item in items {
                        let preview = Preview::from_rss_item(rss_feed_url.clone(), item)?;
                        new_previews.push(preview);
                    }
                }
                Err(e) => {
                    log::warn!["I failed to fetch RSS feed {rss_feed_url}: {e}"];
                }
            }
        }
    }

    log::info!["processing all new previews"];
    for new_preview in new_previews {
        let mut new_preview = new_preview;
        if !exists_preview(&mut env.conn, &new_preview.url)? {
            let url = new_preview.url.clone();
            if let Err(e) = fill_initial_preview(&mut env, &mut new_preview).await {
                log::warn!["Failed to fill initial preview for {url}: {e}"];
            }
            insert_preview(&mut env.conn, &new_preview)?;
        }
    }

    log::info!["collecting all recent previews into RSS channel"];
    {
        let output_previews = get_recent_previews(&mut env.conn, RECENCY_CUTOFF)?;
        let channel = create_rss_channel(output_previews);
        write_rss_channel(channel)?;
    }

    if true {
        log::info!["Clearing saved URLs"];
        clear_saved_urls()?;
    }

    log::info!["End fetching URLs"];

    Ok(())
}
