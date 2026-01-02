use anyhow::Result;
use chrono::Days;
use diesel::prelude::*;
use dotenvy::dotenv;
use futures::future::join_all;
use linkstitcher::{Env, config, embellish_preview, models::Preview, rss_channel, utility};

const FEED_FILENAME: &str = "hackernews.feed.xml";
const FEED_TITLE: &str = "linkstitcher/hackernews";
const FEED_DESCRIPTION: &str = "The linkstitcher feed for Hacker News";
const RECENCY_CUTOFF_DAYS: Days = Days::new(7);

lazy_static::lazy_static! {
    static ref KEYWORDS: Vec<String> = [
        "programming languages",
        "type theory",
        "type system",
        "haskell",
        "AI",
        "developer tools",
        "video game development",
        "functional programming",
        "dev tools",
        "rust",
        "purescript",
        "compilers",
        "developer experience",
        "category theory",
        "liquid haskell",
        "monad",
        "metaprogramming",
        "mac mini",
        "logic programming",
        "effect systems for purely functional programming langauges",
        "typescript",
        "ocaml",
        "rust",
        "purescript",
        "compiler",
        "mcp",
        "prediction market",
        "homotopy"
        ].into_iter().map(|s| s.to_owned()).collect();
    static ref TOPICS: Vec<String> = [
        "haskell",
        "functional",
        "google",
        "software",
        "korea",
        "japan",
        "singapore",
        "palantir",
        "math",
        "meta",
        "gwern",
        "type",
        "lang",
        "syntax",
        "semantics",
        "github"
        ].into_iter().map(|s| s.to_owned()).collect();
}
const SOURCE: &str = "Hackernews: Customized";

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();
    dotenv()?;

    log::trace!("hackernews::main");

    let mut env = Env::new()?;

    // fetch previews from remote RSS channel
    let channel_url = "https://hnrss.org/best";
    let channel = utility::rss::fetch_rss_channel(&env.client, channel_url).await?;
    let mut previews = {
        let raw_previews = rss_channel::into_previews(channel)?;
        let mut previews = vec![];
        for preview in raw_previews {
            let mut preview = preview;
            if utility::db::is_url_known(&mut env.db_conn, &preview.url)? {
                continue;
            }

            preview.source = Some(SOURCE.to_owned());
            previews.push(preview);
        }
        previews
    };

    // embellish previews
    for preview in previews.iter_mut() {
        if let Err(e) = embellish_preview(&mut env, preview).await {
            log::error!("Error during embellish_preview: {e}");
        }
    }

    // filter previews
    let smart_filter = {
        let mut smart_filter = rss_channel::SmartFilter::default();
        smart_filter.add_keywords(KEYWORDS.clone());
        smart_filter.add_topics(TOPICS.clone());
        smart_filter
    };
    let previews = join_all(
        previews
            .into_iter()
            .map(|preview| smart_filter.checked(preview)),
    )
    .await
    .into_iter()
    .filter_map(|item| match item {
        Ok((_, preview)) => Some(preview),
        Err(_) => None,
    })
    .collect::<Vec<_>>();

    // insert previews into database
    for preview in &previews {
        if let Err(e) = utility::db::insert_preview(&mut env.db_conn, preview) {
            log::warn!("{e}");
        }
    }

    // write local RSS channel
    {
        use linkstitcher::schema::previews::dsl;

        let then = chrono::Utc::now()
            .date_naive()
            .checked_sub_days(RECENCY_CUTOFF_DAYS)
            .unwrap();

        let previews = dsl::previews
            .filter(dsl::added_date.gt(then))
            .filter(dsl::source.eq(SOURCE))
            .select(Preview::as_select())
            .load(&mut env.db_conn)?;
        utility::rss::write_rss_channel(
            &[config::FEEDS_DIRPATH, FEED_FILENAME].join("/"),
            utility::rss::create_rss_channel(FEED_TITLE, FEED_DESCRIPTION, previews),
        )?;
    }

    Ok(())
}
