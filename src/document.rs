use serde::{Deserialize, Serialize};
use display_json::{DisplayAsJsonPretty};

#[derive(Serialize, Deserialize, DisplayAsJsonPretty)]
pub struct DocumentData {
    pub title: String,
    pub date: String,
    pub keywords: Vec<String>,
    pub content: String,
}
