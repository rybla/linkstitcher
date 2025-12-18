use anyhow::Result;
use dotenvy::dotenv;
use linkstitcher::{Env, utility};

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();
    dotenv()?;

    log::trace!("bookmarks::main");

    let mut env = Env::new()?;

    let previews = utility::db::get_all_previews(&mut env.db_conn)?;
    for preview in previews {
        println!("- {:?}: {:?}", preview.url, preview.tags());
    }

    Ok(())
}
