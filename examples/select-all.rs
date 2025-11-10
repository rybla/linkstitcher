use anyhow::Result;
use chrono::Days;
use dotenvy::dotenv;
use linkstitcher::*;

#[tokio::main]
async fn main() -> Result<()> {
    dotenv()?;

    let mut env = Env::new()?;
    let previews = get_recent_previews(&mut env.conn, Days::new(10))?;
    println!("previews: {previews:#?}");
    Ok(())
}
