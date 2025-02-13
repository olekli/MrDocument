use display_json::DisplayAsJsonPretty;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Serialize, Deserialize, DisplayAsJsonPretty)]
pub struct DocumentData {
    pub content: Option<String>,
    pub summary: String,
    pub class: String,
    pub source: String,
    pub keywords: Vec<String>,
    pub title: String,
    pub date: String,
}

impl DocumentData {
    pub fn make_filename(&self, suffix: &str) -> String {
        format!("{}-{}.{}", self.date, self.title, suffix)
    }

    pub fn make_path(&self) -> PathBuf {
        PathBuf::from(format!(
            "{}/{}",
            self.class.to_lowercase(),
            self.source.to_lowercase()
        ))
    }
}
