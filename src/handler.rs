use std::path::PathBuf;
use crate::error::Result;

pub async fn handle_new_file(path: PathBuf) -> Result<()> {
    log::info!("handle {path:?}");
    Ok(())
}
