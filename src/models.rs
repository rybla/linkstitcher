use crate::schema::*;
use anyhow::{Result, anyhow};
use chrono::NaiveDate;
use diesel::{prelude::*, sqlite};

#[derive(Debug, Clone, Queryable, Selectable, Insertable)]
#[diesel(table_name = previews)]
#[diesel(check_for_backend(sqlite::Sqlite))]
pub struct Preview {
    // required
    pub url: String,
    pub added_date: NaiveDate,
    pub saved: bool,
    pub embellished: bool,
    pub bookmarked: bool,
    // optional
    pub source: Option<String>,
    pub title: Option<String>,
    pub published_date: Option<String>,
    pub tags: Option<String>,
    pub summary: Option<String>,
}

impl Preview {
    pub fn tags(&self) -> Option<Vec<&str>> {
        self.tags
            .as_ref()
            .map(|tags| tags.split(",").map(|s| s.trim()).collect())
    }

    pub fn from_rss_item(source: String, item: rss::Item) -> Result<Preview> {
        Ok(Preview {
            url: match &item.link {
                None => {
                    return Err(anyhow!(
                        "I can't get the url of this rss item since it doesn't have one: {item:?}"
                    ));
                }
                Some(url) => url.to_owned(),
            },
            added_date: chrono::Utc::now().date_naive(),
            title: item.title,
            source: Some(source.to_string()),
            published_date: item.pub_date,
            tags: {
                let cs: Vec<_> = item
                    .categories
                    .iter()
                    .map(|c| c.name().to_owned())
                    .collect();
                if cs.is_empty() {
                    None
                } else {
                    Some(cs.join(", "))
                }
            },
            summary: item.description,
            bookmarked: false,
            embellished: false,
            saved: false,
        })
    }

    pub fn from_url(url: String) -> Self {
        Self {
            url,
            added_date: chrono::Utc::now().date_naive(),
            title: None,
            source: None,
            published_date: None,
            tags: None,
            summary: None,
            bookmarked: false,
            embellished: false,
            saved: false,
        }
    }
}

impl From<Preview> for rss::Item {
    fn from(val: Preview) -> Self {
        rss::ItemBuilder::default()
            .link(val.url)
            .title(val.title)
            .pub_date(val.added_date.format("%Y-%m-%d").to_string())
            .description(val.summary)
            .build()
    }
}
