use crate::error::{Result, Error};
use tokio::fs;
use std::path::PathBuf;
use uuid::Uuid;
use tokio::fs::create_dir_all;

pub fn file_exists(path: &PathBuf) -> bool {
    if let Ok(exists) = std::fs::exists(&path) {
        exists
    } else {
        false
    }
}

pub async fn make_unique_path(path: PathBuf, filename: String) -> Result<PathBuf> {
    let path = path.clone();
    create_dir_all(path.clone()).await?;
    let mut split = filename.split(".");
    let filename_stem = split.next().expect("Filename needs to have extension");
    let filename_ext = split.next().expect("Filename needs to have extension");
    let mut filename = format!("{}.{}", filename_stem, filename_ext);

    if let Err(e) = fs::File::create_new(path.join(filename.clone())).await {
        log::trace!("unable to create file: {filename}: {e:?}");
        filename = format!("{}-{}.{}", filename_stem, Uuid::new_v4(), filename_ext);
        if let Ok(_) = fs::File::create_new(path.join(filename.clone())).await {
            Ok(path.join(filename))
        } else {
            Err(Error::UnexpectedError("Cannot find unique filename".to_string()))
        }
    } else {
        Ok(path.join(filename))
    }
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
