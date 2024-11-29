use display_json::DisplayAsJsonPretty;
use serde::{Deserialize, Serialize};

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
        format!("{}-{}-{}-{}.{}", self.date, self.class.to_lowercase(), self.source, self.title, suffix)
    }
}
