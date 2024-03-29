use std::env;

use camino::Utf8PathBuf;
use dashmap::DashMap;

use notify::{INotifyWatcher, Watcher as NotifyWatcher};
use tokio::sync::mpsc::UnboundedSender;

use crate::types::{self, BuilderIncomingMessages, HashedFileRecord, Watcher, WatcherError};

#[derive(Default)]
pub struct SimpleWatcher {
    watchers: DashMap<Utf8PathBuf, INotifyWatcher>,
    code_subscribers:
        DashMap<Utf8PathBuf, DashMap<usize, UnboundedSender<BuilderIncomingMessages>>>,
    asset_subscribers:
        DashMap<Utf8PathBuf, DashMap<usize, UnboundedSender<BuilderIncomingMessages>>>,
}

impl Watcher for SimpleWatcher {
    fn watch_code_directories(
        &self,
        directories: &[camino::Utf8PathBuf],
        subscriber: (usize, UnboundedSender<types::BuilderIncomingMessages>),
    ) -> Result<(), WatcherError> {
        for directory in directories.iter() {
            let subscribers = self.code_subscribers.entry(directory.clone()).or_default();
            subscribers.insert(subscriber.0, subscriber.1.clone());
            let _ = self
                .watchers
                .entry(directory.clone())
                .or_try_insert_with::<WatcherError>(|| {
                    let code_subscribers = self.code_subscribers.clone();
                    let directory = directory.clone();

                    let mut watcher = {
                        let directory = directory.clone();
                        notify::recommended_watcher(move |_| {
                            let Some(subscribers) = code_subscribers.get(&directory) else {
                                return;
                            };
                            let _ = subscribers.iter().map(|subscriber| {
                                subscriber.send(BuilderIncomingMessages::CodeChanged)
                            });
                        })?
                    };

                    watcher.watch(directory.as_std_path(), notify::RecursiveMode::Recursive)?;

                    Ok(watcher)
                })?;
        }
        Ok(())
    }

    fn watch_asset_directories(
        &self,
        directories: &[Utf8PathBuf],
        subscriber: (usize, UnboundedSender<types::BuilderIncomingMessages>),
    ) -> Result<(), WatcherError> {
        let cwd = Utf8PathBuf::try_from(env::current_dir()?)?;
        for directory in directories.iter() {
            let subscribers = self.asset_subscribers.entry(directory.clone()).or_default();
            subscribers.insert(subscriber.0, subscriber.1.clone());
            let cwd = cwd.clone();
            let _ = self
                .watchers
                .entry(directory.clone())
                .or_try_insert_with::<WatcherError>(move || {
                    let asset_subscribers = self.asset_subscribers.clone();
                    let directory = directory.clone();

                    let mut watcher = {
                        let directory = directory.clone();

                        notify::recommended_watcher(
                            move |file: Result<notify::Event, notify::Error>| {
                                let Some(subscribers) = asset_subscribers.get(&directory) else {
                                    return;
                                };
                                if let Ok(file) = file {
                                    let files = file
                                        .paths
                                        .iter()
                                        .filter_map(|p| {
                                            Utf8PathBuf::try_from(p.clone())
                                                .map_err(WatcherError::from)
                                                .and_then(|path| {
                                                    if path.is_file() {
                                                        Ok(path)
                                                    } else {
                                                        Err(WatcherError::OtherError(format!(
                                                            "Path is not a file {path}"
                                                        )))
                                                    }
                                                })
                                                .and_then(|path| {
                                                    std::fs::read(&path)
                                                        .map(|file| {
                                                            let hash = blake3::hash(&file);
                                                            let relative_path = path
                                                                .strip_prefix(&cwd)
                                                                .map(|p| p.to_owned())
                                                                .unwrap_or_else(|_| path.clone());
                                                            let record = HashedFileRecord {
                                                                relative_path,
                                                                local_path: path.clone(),
                                                                hash: hash.as_bytes().to_owned(),
                                                            };
                                                            record
                                                        })
                                                        .map_err(WatcherError::from)
                                                })
                                                .ok()
                                        })
                                        .collect::<Vec<_>>();
                                    let _ = subscribers.iter().map(|subscriber| {
                                        for file in files.iter() {
                                            let _ = subscriber.send(
                                                BuilderIncomingMessages::AssetChanged(file.clone()),
                                            );
                                        }
                                    });
                                }
                            },
                        )?
                    };

                    watcher.watch(directory.as_std_path(), notify::RecursiveMode::Recursive)?;

                    Ok(watcher)
                })?;
        }
        Ok(())
    }
}
