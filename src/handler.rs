use crate::chatgpt::query_ai;
use crate::error::{Error, Result};
use crate::file::{FileObject, Location, Paths};
use crate::file_info::FileInfo;
use crate::pdf::update_metadata;
use std::io::Write;
use std::path::PathBuf;
use tokio::fs;
use tokio::fs::create_dir_all;
use tokio::io::AsyncWriteExt;
use tokio::task::JoinSet;
use tokio::time::{sleep, Duration};

pub struct Handler {
    paths: Paths,
    tasks: JoinSet<()>,
    concurrency: u8,
}

impl Handler {
    pub async fn new(paths: Paths, concurrency: u8) -> Result<Self> {
        create_dir_all(paths.make_root(Location::Inbox)).await?;
        create_dir_all(paths.make_root(Location::Outbox)).await?;
        create_dir_all(paths.make_root(Location::Transit)).await?;
        create_dir_all(paths.make_root(Location::Processed)).await?;
        create_dir_all(paths.make_root(Location::Error)).await?;
        Ok(Handler {
            paths,
            tasks: JoinSet::new(),
            concurrency,
        })
    }

    pub async fn handle_file(&mut self, filepath: PathBuf) {
        while self.tasks.len() >= self.concurrency.into() {
            self.tasks
                .join_next()
                .await
                .expect("Cannot be empty")
                .expect("Task should not panic");
        }
        self.tasks.spawn(Handler::handle_file_entry_point(
            self.paths.clone(),
            filepath.clone(),
        ));
    }

    async fn handle_file_entry_point(paths: Paths, filepath: PathBuf) {
        log::info!("Processing {filepath:?}");
        match Handler::handle_file_transit(paths, filepath.clone()).await {
            Ok(_) => {
                log::info!("Processed {:?}", filepath);
            }
            Err(err) => {
                log::error!("Unable to process file: {:?}: {}", filepath, err);
            }
        }
    }

    async fn handle_file_transit(paths: Paths, filepath: PathBuf) -> Result<()> {
        let mut file = FileObject::new(paths, filepath)?;
        log::debug!("Processing as {file:?}");
        match Handler::handle_file_processing(&mut file).await {
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
        log::debug!("Waiting for file");
        sleep(Duration::from_secs(1)).await;
        FileInfo::new(file.get_path())?;
        Handler::wait_for_document(file).await?;
        file.rename(Location::Transit).await?;

        let file_info = FileInfo::new(file.get_path())?;
        let document_data = query_ai(file_info).await?;
        let dst_file_name_pdf = format!(
            "{}-{}.pdf",
            document_data.date.clone(),
            document_data.title.clone()
        );
        let dst_path_pdf = file.make_path_with_new_filename(Location::Outbox, dst_file_name_pdf).await;
        update_metadata(file.get_path(), dst_path_pdf, &document_data)
            .await
            .map(|_| ())?;

        let dst_file_name_txt = format!("{}-{}.txt", document_data.date, document_data.title);
        let dst_path_txt = file.make_path_with_new_filename(Location::Outbox, dst_file_name_txt).await;
        let mut txt_file = fs::File::create(dst_path_txt).await?;
        let mut buffer = Vec::<u8>::new();
        write!(buffer, "{}", document_data.content)?;
        txt_file.write_all(&buffer).await?;

        file.rename(Location::Processed).await?;

        Ok(())
    }

    async fn wait_for_document(file: &FileObject) -> Result<()> {
        let mut i = 6;
        while let Err(_) = lopdf::Document::load(file.get_path()).await {
            if tokio::fs::metadata(file.get_path()).await.is_err() {
                return Err(Error::FileDisappearedError(file.get_path()));
            }
            log::info!("waiting for document to become ready: {file:?}");
            sleep(Duration::from_secs(10)).await;
            i = i - 1;
            if i == 0 {
                return Err(Error::NotValidPdfError);
            }
        }
        Ok(())
    }
}
