use crate::error::{Error, Result};
use futures::stream::SelectAll;
use notify::{
    recommended_watcher, Event, RecommendedWatcher, RecursiveMode,
    Watcher as NotifyWatcher,
};
use std::path::PathBuf;
use tokio::signal::unix::{signal, SignalKind};
use tokio::sync::mpsc;
use tokio_stream::wrappers::{ReceiverStream, SignalStream};
use tokio_stream::Stream;
use tokio_stream::StreamExt;
use tokio::task;
use crate::handler::EventHandler;

pub enum WatcherEvent {
    Event(Event),
    Quit,
    Error(Error),
}

pub struct WatcherLoop {
    shutdown_tx: mpsc::Sender<()>,
    join_handle: task::JoinHandle<Result<()>>,
}

impl WatcherLoop {
    pub async fn new<H>(path: PathBuf, event_handler: H) -> Result<Self>
    where
        H: EventHandler,
    {
        let (watcher, shutdown_tx) = Watcher::new(path)?;
        let join_handle = tokio::task::spawn(WatcherLoop::run(watcher, event_handler));

        Ok(WatcherLoop{
            shutdown_tx,
            join_handle,
        })
    }

    pub async fn shutdown(self) -> Result<()> {
        self.shutdown_tx.send(()).await?;

        Ok(self.join_handle.await??)
    }

    pub async fn wait(self) -> Result<()> {
        Ok(self.join_handle.await??)
    }

    async fn run<H>(mut watcher: Watcher, mut event_handler: H) -> Result<()>
    where
        H: EventHandler,
    {
        loop {
            match watcher.queue.next().await {
                Some(event) => match event {
                    WatcherEvent::Event(event) => {
                        event_handler.handle_event(event).await;
                    }
                    WatcherEvent::Quit => {
                        log::info!("Received signal. Exiting.");
                        break Ok(());
                    }
                    WatcherEvent::Error(err) => {
                        log::error!("{err:?}");
                        break Err(err);
                    }
                },
                None => {
                    break Err(Error::StreamClosedError);
                }
            };
        }
    }
}

struct Watcher {
    queue: Box<dyn Stream<Item = WatcherEvent> + Send + Unpin>,

    _watcher: RecommendedWatcher,
}

impl Watcher {
    fn new(path: PathBuf) -> Result<(Self, mpsc::Sender<()>)> {
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

        let (shutdown_tx, shutdown_rx) = mpsc::channel(100);

        let shutdown_stream = ReceiverStream::new(shutdown_rx).map(|()| Some(WatcherEvent::Quit));
        let notify_stream = ReceiverStream::new(notify_rx).map(Watcher::filter_events);

        let queue = Box::new(notify_stream.merge(signal_stream).merge(shutdown_stream).filter_map(|e| e));

        Ok((Watcher {
            _watcher: watcher,
            queue,
        }, shutdown_tx))
    }

    fn filter_events(event: std::result::Result<Event, notify::Error>) -> Option<WatcherEvent> {
        match event {
            Ok(event) => Some(WatcherEvent::Event(event)),
            Err(err) => {
                log::warn!("Dropping error event: {err:?}");
                None
            }
        }
    }
}
