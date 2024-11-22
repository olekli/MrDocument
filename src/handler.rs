use crate::error::{Result};
use crate::file_info::FileInfo;
use crate::chatgpt::query_ai;
use crate::pdf::update_metadata;
use std::path::PathBuf;
use crate::paths::{Paths, Location};
use tokio::fs;

pub async fn handle_file(paths: &Paths, path: PathBuf) {
    match handle_file_transit(paths, path.clone()).await {
        Err(err) => {
            log::error!("Unable to process file: {:?}: {}", path, err);
        }
        _ => {}
    }
}

async fn handle_file_transit(paths: &Paths, path: PathBuf) -> Result<()> {
    match handle_file_processing(paths, path.clone()).await {
        Ok(_) => Ok(()),
        Err(err) => {
            let error_path = paths.make_path(Location::Error, path.clone())?;
            if let Err(err) = fs::rename(path.clone(), error_path).await {
                log::error!("Unable to move file to error location: {:?}: {}", path, err);
            }
            Err(err)
        }
    }
}

async fn handle_file_processing(paths: &Paths, path: PathBuf) -> Result<()> {
    let transit_path = paths.make_path(Location::Transit, path.clone())?;
    fs::rename(path, transit_path.clone()).await?;
    let file_info = FileInfo::new(transit_path.clone())?;
    let document_data = query_ai(file_info).await?;
    let dst_file_name = PathBuf::from(format!("{}-{}.pdf", document_data.date.clone(), document_data.title.clone()));
    let dst_path = paths.make_path(Location::Outbox, dst_file_name)?;
    update_metadata(transit_path.clone(), dst_path, document_data).await.map(|_| ())?;
    fs::rename(transit_path.clone(), paths.make_path(Location::Processed, transit_path)?).await?;

    Ok(())
}
