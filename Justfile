fetch-urls:
  RUST_LOG=fetch-urls,linkstitcher cargo run --bin fetch-urls

fill-bookmarks:
  RUST_LOG=fill-bookmarks,linkstitcher cargo run --bin fill-bookmarks

deploy: fetch-urls
  (git add site ; git commit -m"fetch-urls: update") || echo "fetch-urls: no updates"
  git subtree split --prefix=site -b gh-pages
  git push -f origin gh-pages

all: deploy fill-bookmarks
