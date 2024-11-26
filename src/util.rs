use crate::error::{Result, Error};
use tokio::fs;
use std::path::PathBuf;
use uuid::Uuid;

pub fn file_exists(path: &PathBuf) -> bool {
    if let Ok(exists) = std::fs::exists(&path) {
        exists
    } else {
        false
    }
}

pub async fn make_unique_path(path: PathBuf, filename: String) -> PathBuf {
    let path = path.clone();
    let mut split = filename.split(".");
    let filename_stem = split.next().expect("Filename needs to have extension");
    let filename_ext = split.next().expect("Filename needs to have extension");
    let mut filename = format!("{}.{}", filename_stem, filename_ext);
    while let Err(_) = fs::File::create_new(path.join(filename.clone())).await {
        filename = format!("{}-{}.{}", filename_stem, Uuid::new_v4(), filename_ext)
    }

    path.join(filename)
}

pub async fn move_file(from: &PathBuf, to: &PathBuf) -> Result<()> {
    fs::File::create_new(to).await?;
    let result = fs::rename(from, to).await;

    match result {
        Ok(_) => Ok(()),
        Err(err) => {
            fs::remove_file(to).await.or_else(|_| Ok::<(), Error>(()))?;
            Err(Error::from(err))
        }
    }
}
