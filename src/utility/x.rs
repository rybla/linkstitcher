use anyhow::Result;

#[derive(Debug, Clone, serde::Deserialize)]
pub struct Post {
    pub url: String,
    pub author_name: String,
    pub author_url: String,
    pub html: String,
}

pub async fn fetch_post(post_url: &str) -> Result<Post> {
    let post_url = urlencoding::encode(post_url);
    let result = reqwest::get(format!("https://publish.twitter.com/oembed?url={post_url}")).await?;
    let post = result.json::<Post>().await?;
    Ok(post)
}
