use crate::{config, models::*};
use anyhow::Result;
use diesel::prelude::*;

pub fn establish_connection() -> SqliteConnection {
    SqliteConnection::establish(&config::DATABASE_URL)
        .unwrap_or_else(|_| panic!("Error connecting to {}", config::DATABASE_URL.as_str()))
}

pub fn insert_or_update_preview(db_conn: &mut SqliteConnection, preview: &Preview) -> Result<()> {
    if is_url_known(db_conn, &preview.url)? {
        update_preview(db_conn, preview)?;
    } else {
        insert_preview(db_conn, preview)?;
    }

    Ok(())
}

pub fn insert_preview(db_conn: &mut SqliteConnection, preview: &Preview) -> Result<()> {
    use crate::schema::previews::dsl;

    diesel::insert_into(dsl::previews)
        .values(preview)
        .execute(db_conn)?;
    Ok(())
}

pub fn update_preview(db_conn: &mut SqliteConnection, preview: &Preview) -> Result<()> {
    use crate::schema::previews::dsl;

    diesel::update(dsl::previews.find(&preview.url))
        .set((
            dsl::source.eq(&preview.source),
            dsl::title.eq(&preview.title),
            dsl::published_date.eq(&preview.published_date),
            dsl::tags.eq(&preview.tags),
            dsl::summary.eq(&preview.summary),
            dsl::embellished.eq(&preview.embellished),
            dsl::bookmarked.eq(&preview.bookmarked),
        ))
        .execute(db_conn)?;
    Ok(())
}

pub fn get_preview(db_conn: &mut SqliteConnection, url: String) -> Result<Option<Preview>> {
    use crate::schema::previews::dsl::previews;

    Ok(previews
        .find(url)
        .select(Preview::as_select())
        .first(db_conn)
        .optional()?)
}

pub fn get_all_previews(db_conn: &mut SqliteConnection) -> Result<Vec<Preview>> {
    use crate::schema::previews::dsl::previews;

    Ok(previews.select(Preview::as_select()).load(db_conn)?)
}

pub fn is_url_known(db_conn: &mut SqliteConnection, url: &str) -> Result<bool> {
    use crate::schema::previews::dsl::previews;

    Ok(previews
        .find(url)
        .first::<Preview>(db_conn)
        .optional()?
        .is_some())
}
