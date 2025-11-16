macro_rules! load_env_var {
    ( $name: ident ) => {
        lazy_static::lazy_static! {
            pub static ref $name: String = std::env::var(stringify!($name)).expect(concat!(
                "This environment variable must be set: ",
                stringify!($name)
            ));
        }
    };
}

load_env_var!(DATABASE_URL);
load_env_var!(GITHUB_PERSONAL_ACCESS_TOKEN);
load_env_var!(BOOKMARKED_URLS_FILEPATH);
load_env_var!(SAVED_URLS_FILEPATH);
pub const REPOSITORY_URL: &str = "https://github.com/rybla/linkstitcher";
pub const FEED_FILEPATH: &str = "site/feed.xml";
pub const RECENCY_CUTOFF: chrono::Days = chrono::Days::new(2);
pub const MAX_RSS_FEED_ITEMS: usize = 5;
pub const MAX_CHARS_SUMMARY: usize = 600;
