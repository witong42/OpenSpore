use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct MemoryItem {
    pub filename: String,
    pub content: String,
}

#[derive(Debug, Clone)]
pub struct SearchResult {
    pub title: String,
    pub content: String,
    pub score: usize,
    pub path: PathBuf,
}
