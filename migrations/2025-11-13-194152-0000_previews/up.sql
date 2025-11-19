CREATE TABLE previews (
  -- required
  url TEXT NOT NULL PRIMARY KEY,
  added_date DATE NOT NULL,
  saved BOOL NOT NULL DEFAULT FALSE,
  embellished BOOL NOT NULL DEFAULT FALSE,
  bookmarked BOOL NOT NULL DEFAULT FALSE,
  -- optional
  source TEXT,
  title TEXT,
  published_date TEXT,
  tags TEXT,
  summary TEXT
)
