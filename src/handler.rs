use crate::chatgpt::query_ai;
use crate::error::{Error, Result};
use crate::file_info::FileInfo;
use crate::file_object::FileObject;
use crate::paths::Location;
use crate::pdf::update_metadata;
use crate::profile::Profile;
use notify::event::CreateKind;
use notify::{Event, EventKind};
use std::future::Future;
use std::marker::Send;
use std::path::PathBuf;
use tokio::fs;
use tokio::fs::create_dir_all;
use tokio::io::AsyncWriteExt;
use tokio::task::JoinSet;
use tokio::time::{sleep, Duration};

pub trait EventHandler: Send + 'static {
    fn handle_event(&mut self, event: Event) -> impl Future<Output = ()> + Send;
    fn on_start(&mut self) -> impl Future<Output = ()> + Send {
        async {}
    }
    fn on_stop(self) -> impl Future<Output = ()> + Send
    where
        Self: Sized,
    {
        async {}
    }
}

pub struct Handler {
    profile: Profile,
    tasks: JoinSet<()>,
    concurrency: u8,
}

impl EventHandler for Handler {
    async fn handle_event(&mut self, event: Event) {
        match event {
            Event {
                kind: EventKind::Create(CreateKind::Any),
                paths,
                ..
            }
            | Event {
                kind: EventKind::Create(CreateKind::File),
                paths,
                ..
            } => {
                let existing_paths: Vec<_> =
                    paths.into_iter().filter(|path| path.is_file()).collect();
                for path in existing_paths {
                    self.handle_file(path).await;
                }
            }
            _ => {
                log::trace!("Ignoring event: {event:?}");
            }
        };
    }

    async fn on_stop(self) {
        self.wait().await;
    }
}

impl Handler {
    pub async fn new(profile: Profile, concurrency: u8) -> Result<Self> {
        create_dir_all(profile.paths.make_root(Location::Inbox)).await?;
        create_dir_all(profile.paths.make_root(Location::Outbox)).await?;
        create_dir_all(profile.paths.make_root(Location::Transit)).await?;
        create_dir_all(profile.paths.make_root(Location::Processed)).await?;
        create_dir_all(profile.paths.make_root(Location::Error)).await?;
        Ok(Handler {
            profile,
            tasks: JoinSet::new(),
            concurrency,
        })
    }

    async fn handle_file(&mut self, filepath: PathBuf) {
        while self.tasks.len() >= self.concurrency.into() {
            self.tasks
                .join_next()
                .await
                .expect("Cannot be empty")
                .expect("Task should not panic");
        }
        self.tasks.spawn(Handler::handle_file_entry_point(
            self.profile.clone(),
            filepath.clone(),
        ));
    }

    async fn wait(self) {
        self.tasks.join_all().await;
    }

    async fn handle_file_entry_point(profile: Profile, filepath: PathBuf) {
        log::info!("Processing {filepath:?}");
        match Handler::handle_file_transit(profile, filepath.clone()).await {
            Ok(_) => {
                log::info!("Processed {:?}", filepath);
            }
            Err(err) => {
                log::error!("Unable to process file: {:?}: {}", filepath, err);
            }
        }
    }

    async fn handle_file_transit(profile: Profile, filepath: PathBuf) -> Result<()> {
        let mut file = FileObject::new(profile.paths.clone(), filepath)?;
        log::debug!("Processing as {file:?}");
        match Handler::handle_file_processing(profile, &mut file).await {
            Ok(_) => Ok(()),
            Err(err) => {
                if let Err(err) = file.rename(Location::Error).await {
                    log::error!("Unable to move file to error location: {:?}: {}", file, err);
                }
                Err(err)
            }
        }
    }

    async fn handle_file_processing(profile: Profile, file: &mut FileObject) -> Result<()> {
        log::debug!("Waiting for file");
        sleep(Duration::from_secs(1)).await;
        FileInfo::new(file.get_path())?;
        Handler::wait_for_document(file).await?;
        file.rename(Location::Transit).await?;

        let file_info = FileInfo::new(file.get_path())?;
        let (classes, sources) = Handler::determine_classes_sources(&profile).await?;
        let document_data = query_ai(profile.chatgpt, file_info, classes, sources).await?;
        let dst_path_pdf = file
            .make_path_with_new_filename(
                Location::Outbox,
                document_data.make_path(),
                document_data.make_filename("pdf"),
            )
            .await?;
        update_metadata(file.get_path(), dst_path_pdf, &document_data)
            .await
            .map(|_| ())?;

        if let Some(ref content) = document_data.content {
            let content_path = file
                .make_path_with_new_filename(
                    Location::Outbox,
                    document_data.make_path(),
                    document_data.make_filename("content"),
                )
                .await?;
            let mut out = fs::File::create(content_path).await?;
            out.write_all(content.as_bytes()).await?;
        }
        let summary_path = file
            .make_path_with_new_filename(
                Location::Outbox,
                document_data.make_path(),
                document_data.make_filename("summary"),
            )
            .await?;
        let mut out = fs::File::create(summary_path).await?;
        out.write_all(document_data.summary.as_bytes()).await?;

        file.rename(Location::Processed).await?;

        Ok(())
    }

    async fn determine_classes_sources(profile: &Profile) -> Result<(Vec<String>, Vec<String>)> {
        let path = profile.paths.make_root(Location::Outbox);
        let mut first_level_dirs = Vec::new();
        let mut second_level_dirs = Vec::new();

        let mut entries = fs::read_dir(&path).await?;
        while let Some(entry) = entries.next_entry().await? {
            let dir_path = entry.path();
            let metadata = entry.metadata().await?;
            if metadata.is_dir() {
                if let Some(dir_name) = dir_path.file_name() {
                    first_level_dirs.push(dir_name.to_string_lossy().into_owned());
                }

                let mut sub_entries = fs::read_dir(&dir_path).await?;
                while let Some(sub_entry) = sub_entries.next_entry().await? {
                    let sub_dir_path = sub_entry.path();
                    let sub_metadata = sub_entry.metadata().await?;
                    if sub_metadata.is_dir() {
                        if let Some(sub_dir_name) = sub_dir_path.file_name() {
                            second_level_dirs.push(sub_dir_name.to_string_lossy().into_owned());
                        }
                    }
                }
            }
        }

        Ok((first_level_dirs, second_level_dirs))
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
