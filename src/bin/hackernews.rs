use anyhow::Result;
use futures::future::join_all;
use linkstitcher::{Env, config, embellish_preview, rss_channel, utility};

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

#[tokio::main]
async fn main() -> Result<()> {
    log::trace!("hackernews::main");
    let mut env = Env::new()?;

    // fetch previews from remote RSS channel
    let channel_url = "https://hnrss.org/best";
    let channel = utility::rss::fetch_rss_channel(&env.client, channel_url).await?;
    let mut previews = rss_channel::into_previews(channel)?;

    // embellish previews
    for preview in previews.iter_mut() {
        embellish_preview(&mut env, preview).await?;
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
        utility::db::insert_preview(&mut env.db_conn, preview)?;
    }

    // write local RSS channel
    utility::rss::write_rss_channel(
        &[config::FEEDS_DIRPATH, "hackernews.feed.xml"].join("/"),
        utility::rss::create_rss_channel(
            "linkstitcher/hackernews",
            "The linkstitcher feed for Hacker News",
            previews,
        ),
    )?;

    Ok(())
}
