use anyhow::{Result, anyhow};
use octocrab::Octocrab;
use url::Url;

#[derive(Debug)]
pub struct RepoInfo {
    pub owner: String,
    pub name: String,
    pub readme: Option<String>,
}

pub async fn fetch_repo_info(octocrab: &Octocrab, url_str: &str) -> Result<RepoInfo> {
    let parsed_url = Url::parse(url_str)?;
    let mut segments = parsed_url
        .path_segments()
        .ok_or_else(|| anyhow!("Invalid URL: cannot get path segments"))?;
    let owner = segments
        .next()
        .ok_or_else(|| anyhow!("Invalid URL: missing owner"))?;
    let repo_name = segments
        .next()
        .ok_or_else(|| anyhow!("Invalid URL: missing repo name"))?;
    let readme = octocrab.repos(owner, repo_name).get_readme().send().await?;

    Ok(RepoInfo {
        owner: owner.to_owned(),
        name: repo_name.to_owned(),
        readme: readme.decoded_content(),
    })
}
