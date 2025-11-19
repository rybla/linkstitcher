use crate::{
    models::Preview,
    utility::{self, indent},
};
use anyhow::Result;

pub fn into_previews(channel: rss::Channel) -> Result<Vec<Preview>> {
    let title = channel.title().trim().to_owned();
    channel
        .items
        .into_iter()
        .map(move |item| Preview::from_rss_item(title.clone(), item))
        .collect()
}

#[derive(Debug, Clone, Default)]
pub struct SmartFilter {
    pub keywords: Vec<String>,
    pub topics: Vec<String>,
}

impl SmartFilter {
    pub fn add_keywords(&mut self, mut keywords: Vec<String>) {
        self.keywords.append(&mut keywords);
    }

    pub fn add_topics(&mut self, mut topics: Vec<String>) {
        self.keywords.append(&mut topics);
    }

    pub async fn check(&self, preview: &Preview) -> Result<bool> {
        let summary = match &preview.summary {
            None => return Ok(false),
            Some(summary) => summary,
        };

        if !self.keywords.is_empty() {
            let mut success = false;
            for keyword in &self.keywords {
                if summary.contains(keyword) {
                    success = true;
                    break;
                }
            }
            if !success {
                return Ok(false);
            }
        }

        if !self.topics.is_empty() {
            let response = utility::ai::gemini_cli(&format!(
                "Consider the following passage:\n\n{}\n\nYour task is to decide if the above passage is related to any of the following topics: {}. Respond with yes or no.",
                self.topics.join(", "),
                indent(summary),
            ))?;
            if !(response.contains("yes") || response.contains("Yes")) {
                return Ok(false);
            }
        }

        Ok(true)
    }

    pub async fn checked(&self, preview: Preview) -> Result<(bool, Preview)> {
        let check = self.check(&preview).await?;
        Ok((check, preview))
    }
}
