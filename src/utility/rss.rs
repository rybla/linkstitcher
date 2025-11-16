use crate::{config, models::Preview};
use anyhow::Result;
use std::{fs::File, io::BufWriter};

pub fn create_rss_channel(title: &str, description: &str, previews: Vec<Preview>) -> rss::Channel {
    rss::ChannelBuilder::default()
        .title(title)
        .image(rss::Image {
            url: "https://www.rybl.net/favicon.ico".to_owned(),
            title: "rybl/linkstitcher".to_owned(),
            link: config::REPOSITORY_URL.to_string(),
            width: None,
            height: None,
            description: None,
        })
        .link(config::REPOSITORY_URL.to_string())
        .description(description)
        .items(
            previews
                .into_iter()
                .map(rss::Item::from)
                .collect::<Vec<_>>(),
        )
        .build()
}

pub fn write_rss_channel(file_path: &str, channel: rss::Channel) -> Result<()> {
    let file = File::create(file_path)?;
    let writer = BufWriter::new(file);
    channel.pretty_write_to(writer, b' ', 4)?;
    Ok(())
}

pub async fn fetch_rss_channel(client: &reqwest::Client, url: &str) -> Result<rss::Channel> {
    let content = client.get(url).send().await?;
    let bytes = content.bytes().await?;
    let channel = rss::Channel::read_from(&bytes[..])?;
    Ok(channel)
}
