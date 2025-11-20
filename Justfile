hackernews:
  RUST_LOG=hackernews,linkstitcher cargo run --bin hackernews

saveds:
  RUST_LOG=saveds,linkstitcher cargo run --bin saveds

bookmarks:
  RUST_LOG=bookmarks,linkstitcher cargo run --bin bookmarks

fetch: saveds hackernews bookmarks

deploy:
  (git add site ; git commit -m"deploy: update") || echo "deploy: no updates"
  git subtree split --prefix=site -b gh-pages
  git push -f origin gh-pages

all: fetch deploy
