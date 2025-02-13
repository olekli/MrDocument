use crate::error::Result;
use crate::handler::{EventHandler, Handler};
use crate::paths::Location;
use crate::profile::Profile;
use crate::watcher::WatcherLoop;
use filetime::{set_file_times, FileTime};
use notify::{Event, EventKind};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::SystemTime;
use tokio::fs;
use tokio::io::AsyncReadExt;
use tokio::io::BufReader;
use tokio::task;

pub struct ProfileHandler {
    path: PathBuf,
    profiles: HashMap<PathBuf, (Option<String>, Option<WatcherLoop>)>,
}

impl ProfileHandler {
    pub fn new(path: PathBuf) -> ProfileHandler {
        ProfileHandler {
            path,
            profiles: HashMap::new(),
        }
    }

    async fn on_start_impl(&mut self) -> Result<()> {
        log::debug!("Pinging all profiles");
        let mut dir_entries = fs::read_dir(self.path.clone()).await?;
        while let Some(entry) = dir_entries.next_entry().await? {
            let entry_path = entry.path();
            touch_file(entry_path).await?;
        }
        Ok(())
    }

    async fn make_watcher_loop(path: PathBuf) -> Result<(String, WatcherLoop)> {
        let hash = compute_file_hash(&path).await?;
        let profile = Profile::new_from_file(path.clone()).await?;
        log::info!("Starting watcher on {:?}", profile.paths.path);
        let inbox_path = profile.paths.make_root(Location::Inbox);
        let handler = Handler::new(profile.clone(), 4).await?;
        let watcher_loop = WatcherLoop::new(inbox_path, handler, profile.polling).await?;

        Ok((hash, watcher_loop))
    }

    async fn handle_profile(&mut self, path: PathBuf, event: EventKind) -> Result<()> {
        if self.profiles.contains_key(&path) {
            match event {
                EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_) => {
                    let (hash, _) = self.profiles.get(&path).unwrap();
                    if *hash != compute_file_hash(&path).await.ok() {
                        let (_, running_loop) = self.profiles.remove(&path).unwrap();
                        if let Some(running_loop) = running_loop {
                            running_loop.shutdown().await.inspect_err(|e| {
                                log::warn!("Cannot shutdown running watcher: {path:?}: {e:?}")
                            })?;
                        }
                    }
                }
                _ => {
                    log::trace!("Ignoring {path:?}: {event:?}");
                }
            }
        }
        if !self.profiles.contains_key(&path) && path.is_file() {
            let hash_watcher_loop = ProfileHandler::make_watcher_loop(path.clone())
                .await
                .inspect_err(|e| log::error!("Unable to create watcher for {path:?}: {e:?}"))
                .ok();
            self.profiles.insert(path, hash_watcher_loop.unzip());
        }

        Ok(())
    }
}

impl EventHandler for ProfileHandler {
    async fn on_start(&mut self) {
        self.on_start_impl()
            .await
            .inspect_err(|e| log::warn!("Unable to ping profile files: {e:?}"))
            .ok();
    }

    async fn handle_event(&mut self, event: Event) {
        match event {
            Event { kind, paths, .. } => {
                for path in paths {
                    self.handle_profile(path.clone(), kind)
                        .await
                        .inspect_err(|e| log::error!("Unable to run profile: {path:?}: {e:?}"))
                        .ok();
                }
            }
        };
    }
}

async fn compute_file_hash(path: &PathBuf) -> Result<String> {
    let file = fs::File::open(path).await?;
    let mut reader = BufReader::new(file);
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 4096];
    loop {
        let bytes_read = reader.read(&mut buffer).await?;
        if bytes_read == 0 {
            break;
        }
        hasher.update(&buffer[..bytes_read]);
    }
    let result = hasher.finalize();

    Ok(format!("{:x}", result))
}

async fn touch_file(path: PathBuf) -> Result<()> {
    let metadata = fs::metadata(&path).await?;
    let mtime = FileTime::from_last_modification_time(&metadata);
    let atime = FileTime::from_system_time(SystemTime::now());
    task::spawn_blocking(move || set_file_times(&path, atime, mtime)).await??;

    Ok(())
}
