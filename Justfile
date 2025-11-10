fetch:
  RUST_LOG=fetch,linkstitcher cargo run --bin fetch

deploy: fetch
  (git add site ; git commit -m"fetch: update") || echo "fetch: no updates"
  git subtree split --prefix=site -b gh-pages
  git push -f origin gh-pages
