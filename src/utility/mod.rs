pub mod ai;
pub mod arxiv;
pub mod db;
pub mod github;
pub mod rss;
pub mod x;

pub fn indent(s: &str) -> String {
    s.split("\n")
        .map(|s| format!("    {s}"))
        .collect::<Vec<_>>()
        .join("\n")
}
