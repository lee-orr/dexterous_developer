use std::env;

use camino::{Utf8Path, Utf8PathBuf};
use dashmap::DashMap;

use notify::{INotifyWatcher, Watcher as NotifyWatcher};
use tokio::sync::mpsc::UnboundedSender;
use tracing::{error, info, warn};

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
        info!("Watching Directories: {directories:?}");
        for directory in directories.iter() {
            {
                info!("Checking {directory:?}");
                let subscribers = self.code_subscribers.entry(directory.clone()).or_default();
                info!("Got Subscribers");
                subscribers.insert(subscriber.0, subscriber.1.clone());
            }
            info!("Inserting a new subscriber");
            let _ = self
                .watchers
                .entry(directory.clone())
                .or_try_insert_with::<WatcherError>(|| {
                    info!("Adding watcher entry");
                    let code_subscribers = self.code_subscribers.clone();
                    info!("Getting Code Subscribers");
                    let directory = directory.clone();

                    let mut watcher = {
                        let directory = directory.clone();
                        notify::recommended_watcher(move |_| {
                            info!("Got Watch Event");
                            let Some(subscribers) = code_subscribers.get(&directory) else {
                                error!("Couldn't Get Subscribers");
                                return;
                            };
                            if subscribers.is_empty() {
                                warn!("No Subscribers");
                                return;
                            }
                            for subscriber in subscribers.iter() {
                                info!("Sending Code Changed Message to {}", subscriber.key());
                                let _ = subscriber.send(BuilderIncomingMessages::CodeChanged);
                            }
                            info!("Finished Sending Code Changed Messages");
                        })?
                    };

                    info!("Watching Directory");

                    watcher.watch(directory.as_std_path(), notify::RecursiveMode::Recursive)?;

                    info!("Returning Watcher");

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
        info!("Watching Asset Directories: {directories:?}");
        let cwd = Utf8PathBuf::try_from(env::current_dir()?)?;
        for directory in directories.iter() {
            {
                info!("Checking assets at {directory:?}");
                let subscribers = self.asset_subscribers.entry(directory.clone()).or_default();
                info!("Got asset Subscribers");
                subscribers.insert(subscriber.0, subscriber.1.clone());

                let files = gather_directory_content(directory.clone(), &cwd)?;
                info!("Publishing Current Directory Content {files:?}");

                let _ = subscribers.iter().map(|subscriber| {
                    for file in files.iter() {
                        info!("Sending Asset Changed Message to {}", subscriber.key());
                        let _ =
                            subscriber.send(BuilderIncomingMessages::AssetChanged(file.clone()));
                    }
                });
                for file in files.iter() {
                    info!("Sending Asset Changed Message to {}", subscriber.0);
                    let _ = subscriber
                        .1
                        .send(BuilderIncomingMessages::AssetChanged(file.clone()));
                }
            }
            info!("Inserting a new asset subscriber");
            {
                let cwd = cwd.clone();
                let _ = self
                    .watchers
                    .entry(directory.clone())
                    .or_try_insert_with::<WatcherError>(move || {
                        info!("Adding watcher entry");
                        let asset_subscribers = self.asset_subscribers.clone();
                        info!("Getting asset Subscribers");
                        let directory = directory.clone();

                        let mut watcher = {
                            let directory = directory.clone();

                            notify::recommended_watcher(
                                move |file: Result<notify::Event, notify::Error>| {
                                    info!("Got Asset Event");
                                    let Some(subscribers) = asset_subscribers.get(&directory)
                                    else {
                                        error!("Couldn't Get Asset Subscribers");
                                        return;
                                    };
                                    if subscribers.is_empty() {
                                        warn!("No Asset Subscribers");
                                        return;
                                    }
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
                                                            .map_err(WatcherError::from)
                                                            .and_then(|file| {
                                                                let name =
                                                                    match path.file_name() {
                                                                        Some(n) => n.to_string(),
                                                                        None => return Err(
                                                                            WatcherError::NotAFile(
                                                                                path.clone(),
                                                                            ),
                                                                        ),
                                                                    };
                                                                let hash = blake3::hash(&file);
                                                                let relative_path = path
                                                                    .strip_prefix(&cwd)
                                                                    .map(|p| p.to_owned())
                                                                    .unwrap_or_else(|_| {
                                                                        path.clone()
                                                                    });
                                                                let record = HashedFileRecord::new(
                                                                    relative_path,
                                                                    path.clone(),
                                                                    name,
                                                                    hash.as_bytes().to_owned(),
                                                                );
                                                                Ok(record)
                                                            })
                                                    })
                                                    .ok()
                                            })
                                            .collect::<Vec<_>>();
                                        info!("Asset Change Records: {files:?}");
                                        for subscriber in subscribers.iter() {
                                            info!("Updating Subscriber {}", subscriber.key());
                                            for file in files.iter() {
                                                let _ = subscriber.send(
                                                    BuilderIncomingMessages::AssetChanged(
                                                        file.clone(),
                                                    ),
                                                );
                                            }
                                        }
                                    }
                                },
                            )?
                        };
                        info!("Watching Directory");

                        watcher.watch(directory.as_std_path(), notify::RecursiveMode::Recursive)?;

                        info!("Returning Watcher");

                        Ok(watcher)
                    })?;
            }
        }
        Ok(())
    }
}

fn gather_directory_content(
    dir: Utf8PathBuf,
    cwd: &Utf8Path,
) -> Result<Vec<HashedFileRecord>, std::io::Error> {
    let read = dir.read_dir()?;
    let result = read
        .filter_map(|entry| entry.ok())
        .filter_map(|entry| {
            entry.file_type().ok().and_then(|file_type| {
                if file_type.is_dir() {
                    return Utf8PathBuf::from_path_buf(entry.path())
                        .ok()
                        .map(|path| (path, true));
                }
                if file_type.is_file() {
                    return Utf8PathBuf::from_path_buf(entry.path())
                        .ok()
                        .map(|path| (path, false));
                }
                None
            })
        })
        .filter_map(|(path, is_dir)| {
            if is_dir {
                return gather_directory_content(path, cwd).ok();
            }
            std::fs::read(&path)
                .map_err(WatcherError::from)
                .and_then(|file| {
                    let name = match path.file_name() {
                        Some(n) => n.to_string(),
                        None => return Err(WatcherError::NotAFile(path.clone())),
                    };
                    let hash = blake3::hash(&file);
                    let relative_path = path
                        .strip_prefix(cwd)
                        .map(|p| p.to_owned())
                        .unwrap_or_else(|_| path.clone());
                    let record = HashedFileRecord::new(
                        relative_path,
                        path.clone(),
                        name,
                        hash.as_bytes().to_owned(),
                    );
                    Ok(vec![record])
                })
                .ok()
        })
        .flatten();

    Ok(result.collect())
}

#[cfg(test)]
mod test {
    use std::time::Duration;

    use super::*;
    use test_temp_dir::test_temp_dir;
    use tokio::fs::*;
    use tokio::io::AsyncWriteExt;
    use tokio::sync::mpsc::error::TryRecvError;
    use tokio::sync::mpsc::*;
    use tokio::time::timeout;

    #[tokio::test]
    async fn watcher_can_see_changes_in_a_code_directory() {
        let dir = test_temp_dir!();

        let watcher = SimpleWatcher::default();

        let (tx, mut rx) = unbounded_channel();

        watcher
            .watch_code_directories(
                &[Utf8PathBuf::from_path_buf(dir.as_path_untracked().to_path_buf()).unwrap()],
                (0, tx),
            )
            .expect("Couldn't set up watcher on temporary directory");

        let result = rx.try_recv().expect_err("Should be empty");

        assert!(matches!(result, TryRecvError::Empty));

        let _ = File::create(dir.as_path_untracked().join("test.txt"))
            .await
            .expect("Couldn't create file");

        let result = timeout(Duration::from_millis(10), rx.recv())
            .await
            .expect("Didn't recieve watcher message on time")
            .expect("Didn't recieve watcher message");

        assert!(matches!(result, BuilderIncomingMessages::CodeChanged));
    }

    #[tokio::test]
    async fn watcher_provides_initial_files_in_asset_directory() {
        let dir = test_temp_dir!();

        let watcher = SimpleWatcher::default();

        let (tx, mut rx) = unbounded_channel();

        let _ = File::create(dir.as_path_untracked().join("test.txt"))
            .await
            .expect("Couldn't create file");

        watcher
            .watch_asset_directories(
                &[Utf8PathBuf::from_path_buf(dir.as_path_untracked().to_path_buf()).unwrap()],
                (0, tx),
            )
            .expect("Couldn't set up watcher on temporary directory");

        let result = timeout(Duration::from_millis(10), rx.recv())
            .await
            .expect("Didn't recieve initial asset message on time")
            .expect("Didn't recieve initial asset message");

        let BuilderIncomingMessages::AssetChanged(record) = result else {
            panic!("Got Message that isn't Asset Changed");
        };

        assert_eq!(record.name, "test.txt");
    }

    #[tokio::test]
    async fn watcher_provides_changed_files_in_asset_directory() {
        let dir = test_temp_dir!();

        let watcher = SimpleWatcher::default();

        let (tx, mut rx) = unbounded_channel();

        let _ = File::create(dir.as_path_untracked().join("test.txt"))
            .await
            .expect("Couldn't create file");

        watcher
            .watch_asset_directories(
                &[Utf8PathBuf::from_path_buf(dir.as_path_untracked().to_path_buf()).unwrap()],
                (0, tx),
            )
            .expect("Couldn't set up watcher on temporary directory");

        let result = timeout(Duration::from_millis(10), rx.recv())
            .await
            .expect("Didn't recieve initial asset message on time")
            .expect("Didn't recieve initial asset message");

        let BuilderIncomingMessages::AssetChanged(record) = result else {
            panic!("Got Message that isn't Asset Changed");
        };

        let hash = record.hash;

        assert_eq!(record.name, "test.txt");

        let mut file = File::create(dir.as_path_untracked().join("test.txt"))
            .await
            .expect("Couldn't create file");

        let _ = timeout(Duration::from_millis(10), rx.recv())
            .await
            .expect("Didn't recieve initial asset message on time")
            .expect("Didn't recieve initial asset message");

        file.write_all(b"my file")
            .await
            .expect("Failed to write file");

        let result = timeout(Duration::from_millis(10), rx.recv())
            .await
            .expect("Didn't recieve initial asset message on time")
            .expect("Didn't recieve initial asset message");

        let BuilderIncomingMessages::AssetChanged(record) = result else {
            panic!("Got Message that isn't Asset Changed");
        };

        assert_eq!(record.name, "test.txt");

        assert!(hash != record.hash);
    }
}
