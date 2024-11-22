use crate::error::{Result};
use crate::file_info::FileInfo;
use crate::chatgpt::query_ai;
use crate::pdf::update_metadata;
use std::path::PathBuf;

pub async fn handle_new_file(file_path: PathBuf, outbox_path: PathBuf) -> Result<()> {
    let file_info = FileInfo::new(file_path.clone())?;
    let document_data = query_ai(file_info).await?;
    let dst_file_name = format!("{}-{}.pdf", document_data.date.clone(), document_data.title.clone());
    let dst_path = outbox_path.join(dst_file_name);
    update_metadata(file_path, dst_path, document_data).await.map(|_| ())
}
