use anyhow::Result;
use dotenvy::dotenv;
use linkstitcher::*;

#[tokio::main]
pub async fn main() -> Result<()> {
    env_logger::init();
    dotenv().ok();

    let mut env = Env::new()?;

    let bookmarked_urls = get_bookmarked_urls()?;

    for bookmarked_url in bookmarked_urls {
        let mut preview = Preview::from_url(bookmarked_url.to_string());
        if let Err(e) = fill_bookmarked_preview(&mut env, &mut preview).await {
            log::warn!["Failed to fill bookmarked preview for {}: {e}", preview.url];
        }
        insert_or_update_preview(&mut env.conn, &preview)?;
    }

    if true {
        clear_bookmarked_urls()?;
    }

    Ok(())
}
