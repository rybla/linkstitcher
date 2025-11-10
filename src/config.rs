macro_rules! get_env_var {
    ($name: ident) => {
        std::env::var(stringify!($name)).expect(concat!(
            "This environment variable must be set: ",
            stringify!($name)
        ))
    };
}

lazy_static::lazy_static! {
    pub static ref GITHUB_PERSONAL_ACCESS_TOKEN: String = get_env_var!(GITHUB_PERSONAL_ACCESS_TOKEN);
    pub static ref DATABASE_URL: String = get_env_var!(DATABASE_URL);
    pub static ref URLS_FILEPATH: String = get_env_var!(URLS_FILEPATH);
    pub static ref RSS_FEED_URLS_FILEPATH: String = get_env_var!(RSS_FEED_URLS_FILEPATH);
}

pub const REPOSITORY_URL: &str = "https://github.com/rybla/linkstitcher";
pub const FEED_FILEPATH: &str = "site/feed.xml";
pub const RECENCY_CUTOFF: chrono::Days = chrono::Days::new(7);
pub const MAX_RSS_FEED_ITEMS: usize = 5;
