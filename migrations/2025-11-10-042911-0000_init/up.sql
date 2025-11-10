CREATE TABLE previews (
  url TEXT NOT NULL PRIMARY KEY,
  added_date DATE NOT NULL,
  source TEXT,
  title TEXT,
  published_date TEXT,
  tags TEXT,
  summary TEXT,
  content TEXT
)
