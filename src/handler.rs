use crate::error::{Result};
use crate::file_info::FileInfo;
use crate::chatgpt::query_ai;
use crate::pdf::update_metadata;
use crate::paths::{Location, FileObject};
use tokio::fs;
use tokio::io::AsyncWriteExt;
use std::io::Write;

pub async fn handle_file(mut file: FileObject) {
    match handle_file_transit(&mut file).await {
        Ok(_) => {
            log::info!("Processed {:?}", file);
        }
        Err(err) => {
            log::error!("Unable to process file: {:?}: {}", file, err);
        }
    }
}

async fn handle_file_transit(file: &mut FileObject) -> Result<()> {
    match handle_file_processing(file).await {
        Ok(_) => Ok(()),
        Err(err) => {
            if let Err(err) = file.rename(Location::Error).await {
                log::error!("Unable to move file to error location: {:?}: {}", file, err);
            }
            Err(err)
        }
    }
}

async fn handle_file_processing(file: &mut FileObject) -> Result<()> {
    file.rename(Location::Transit).await?;

    let file_info = FileInfo::new(file.get_path())?;
    let document_data = query_ai(file_info).await?;
    let dst_file_name_pdf = format!("{}-{}.pdf", document_data.date.clone(), document_data.title.clone());
    let dst_path_pdf = file.make_path_with_new_filename(Location::Outbox, dst_file_name_pdf);
    update_metadata(file.get_path(), dst_path_pdf, &document_data).await.map(|_| ())?;

    let dst_file_name_txt = format!("{}-{}.txt", document_data.date, document_data.title);
    let dst_path_txt = file.make_path_with_new_filename(Location::Outbox, dst_file_name_txt);
    let mut txt_file = fs::File::create(dst_path_txt).await?;
    let mut buffer = Vec::<u8>::new();
    write!(buffer, "{}", document_data.content)?;
    txt_file.write_all(&buffer).await?;

    file.rename(Location::Processed).await?;

    Ok(())
}
