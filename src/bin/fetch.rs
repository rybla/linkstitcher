use anyhow::Result;
use dotenvy::dotenv;
use linkstitcher::config::*;
use linkstitcher::*;
use octocrab::Octocrab;
use readability_js::Readability;

#[tokio::main]
pub async fn main() -> Result<()> {
    env_logger::init();
    dotenv().ok();

    let mut conn = establish_connection();

    // if true {
    //     use diesel::dsl::delete;
    //     use diesel::prelude::*;

    //     log::info!["deleting all existing previews"];

    //     use previews::dsl;
    //     delete(dsl::previews.filter(previews::url.is_not("NA"))).execute(&mut conn)?;
    // }

    let readability = Readability::new()?;

    let octocrab = Octocrab::builder()
        .personal_token(config::GITHUB_PERSONAL_ACCESS_TOKEN.as_str())
        .build()
        .unwrap();

    let client = reqwest::Client::new();

    let mut env = ProcessEnv {
        conn: &mut conn,
        readability: &readability,
        octocrab: &octocrab,
        client: &client,
    };

    log::info!["fetching new previews"];

    let mut new_previews: Vec<Preview> = vec![];

    log::info!["fetching new previews from urls"];
    {
        let urls = fetch_urls()?;
        for url in urls {
            new_previews.push(Preview::from_url(url));
        }
    }

    log::info!["fetching new previews from RSS feeds"];
    {
        let rss_feed_urls = fetch_rss_feed_urls()?;
        for rss_feed_url in rss_feed_urls {
            match fetch_rss_channel(env.client, &rss_feed_url).await {
                Ok(channel) => {
                    let mut items = channel.items;
                    items.truncate(MAX_RSS_FEED_ITEMS);
                    for item in items {
                        let preview = Preview::from_rss_item(rss_feed_url.clone(), item)?;
                        new_previews.push(preview);
                    }
                }
                Err(e) => {
                    log::warn!["failed to fetch RSS feed \"{rss_feed_url}\": {e}"];
                }
            }
        }
    }

    log::info!["processing all new previews"];
    for new_preview in new_previews {
        let mut new_preview = new_preview;
        let url = new_preview.url.clone();
        if let Err(e) = process_preview(&mut env, &mut new_preview).await {
            log::warn!["failed to process preview \"{url}\": {e}"];
        }
        set_preview(env.conn, new_preview.clone())?;
    }

    log::info!["collecting all recent previews into RSS channel"];
    {
        let output_previews = get_recent_previews(&mut conn, RECENCY_CUTOFF)?;
        let channel = create_rss_channel(output_previews);
        write_rss_channel(channel)?;
    }

    Ok(())
}
