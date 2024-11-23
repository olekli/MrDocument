use crate::error::{Error, Result};
use crate::util::file_exists;
use futures::stream::SelectAll;
use notify::event::CreateKind;
use notify::{
    recommended_watcher, Event, EventKind, RecommendedWatcher, RecursiveMode,
    Watcher as NotifyWatcher,
};
use std::path::PathBuf;
use tokio::signal::unix::{signal, SignalKind};
use tokio::sync::mpsc;
use tokio_stream::wrappers::{ReceiverStream, SignalStream};
use tokio_stream::Stream;
use tokio_stream::StreamExt;

pub struct Watcher {
    pub queue: Box<dyn Stream<Item = WatcherEvent> + Send + Unpin>,

    _watcher: RecommendedWatcher,
}

pub enum WatcherEvent {
    Paths(Vec<PathBuf>),
    Quit,
    Error(Error),
}

impl Watcher {
    pub fn new(path: PathBuf) -> Result<Self> {
        let (notify_tx, notify_rx) = mpsc::channel(100);
        let mut watcher = recommended_watcher(move |event| match notify_tx.blocking_send(event) {
            Err(err) => {
                log::error!("Cannot send event: {err:?}");
            }
            _ => {}
        })?;
        watcher.watch(&path, RecursiveMode::Recursive)?;

        let signal_stream = vec![
            SignalStream::new(signal(SignalKind::terminate())?),
            SignalStream::new(signal(SignalKind::quit())?),
            SignalStream::new(signal(SignalKind::interrupt())?),
        ]
        .into_iter()
        .collect::<SelectAll<_>>()
        .map(|_| Some(WatcherEvent::Quit));

        let notify_stream = ReceiverStream::new(notify_rx).map(Watcher::filter_events);

        let queue = Box::new(notify_stream.merge(signal_stream).filter_map(|e| e));

        Ok(Watcher {
            _watcher: watcher,
            queue,
        })
    }

    fn filter_events(event: std::result::Result<Event, notify::Error>) -> Option<WatcherEvent> {
        match event {
            Ok(event) => match event {
                Event {
                    kind: EventKind::Create(CreateKind::File),
                    paths,
                    ..
                } => {
                    let existing_paths: Vec<_> = paths
                        .into_iter()
                        .filter(|path| file_exists(&path))
                        .collect();
                    Some(WatcherEvent::Paths(existing_paths))
                }
                _ => {
                    log::trace!("Ignoring event: {event:?}");
                    None
                }
            },
            Err(err) => {
                log::warn!("Dropping error event: {err:?}");
                None
            }
        }
    }
}
