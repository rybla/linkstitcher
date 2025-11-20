use anyhow::Result;
use dotenvy::dotenv;
use linkstitcher::{Env, embellish_preview, models::Preview};

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();
    dotenv()?;

    log::trace!("embellish_url::main");

    let mut env = Env::new()?;

    let mut args: Vec<String> = std::env::args().collect();
    // first arg is exe name; ignore it
    args.remove(0);
    // rest of args are urls to process
    for url in args {
        let mut preview = Preview::from_url(url);
        embellish_preview(&mut env, &mut preview)
            .await
            .unwrap_or_else(|e| {
                println!("{e}");
                None
            });
        println!("------------------------------------------------");
        println!("{preview:#?}");
    }

    Ok(())
}
