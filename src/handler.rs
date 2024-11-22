use crate::error::{Result, Error};
use crate::chatgpt::query_ai;
use crate::pdf::update_metadata;
use crate::file::{Location, FileObject};
use tokio::fs;
use tokio::io::AsyncWriteExt;
use std::io::Write;
use tokio::time::{sleep, Duration};
use crate::file_info::FileInfo;

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

async fn wait_for_document(file: &FileObject) -> Result<()> {
    let mut i = 6;
    while let Err(_) = lopdf::Document::load(file.get_path()).await {
        log::info!("waiting for document to become ready: {file:?}");
        sleep(Duration::from_secs(10)).await;
        i = i - 1;
        if i == 0 {
            return Err(Error::NotValidPdfError);
        }
    }
    Ok(())
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
    sleep(Duration::from_secs(1)).await;

    let file_info = FileInfo::new(file.get_path())?;
    wait_for_document(file).await?;
    file.rename(Location::Transit).await?;

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
