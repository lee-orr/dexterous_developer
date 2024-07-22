use std::env;

use camino::{Utf8Path, Utf8PathBuf};
use dashmap::DashMap;

use notify::{RecommendedWatcher, Watcher as NotifyWatcher};
use tokio::sync::broadcast::{self, Sender};
use tracing::{error, info, trace, warn};

use crate::types::{self, BuilderIncomingMessages, HashedFileRecord, Watcher, WatcherError};

pub struct SimpleWatcher {
    channel: tokio::sync::broadcast::Sender<BuilderIncomingMessages>,
    watchers: DashMap<Utf8PathBuf, RecommendedWatcher>,
}

impl Default for SimpleWatcher {
    fn default() -> Self {
        Self {
            channel: broadcast::channel(100).0,
            watchers: Default::default(),
        }
    }
}

impl Watcher for SimpleWatcher {
    fn watch_code_directories(
        &self,
        directories: &[camino::Utf8PathBuf],
    ) -> Result<(), WatcherError> {
        info!("Watching Directories: {directories:?}");
        for directory in directories.iter() {
            if self.watchers.contains_key(directory) {
                trace!("{directory} already watched");
                continue;
            }
            trace!("Setting up watcher for {directory}");
            let _ = self
                .watchers
                .entry(directory.clone())
                .or_try_insert_with::<WatcherError>(|| {
                    trace!("Adding watcher entry");
                    let directory = directory.clone();

                    let mut watcher = {
                        let channel = self.channel.clone();
                        notify::recommended_watcher(move |_| {
                            info!("Got Watch Event");
                            let _ = channel.send(BuilderIncomingMessages::CodeChanged);
                            trace!("Finished Sending Code Changed Messages");
                        })?
                    };

                    trace!("Watching Directory");

                    watcher.watch(directory.as_std_path(), notify::RecursiveMode::Recursive)?;

                    trace!("Returning Watcher");

                    Ok(watcher)
                })?;
        }
        Ok(())
    }

    fn watch_asset_directories(
        &self,
        directories: &[Utf8PathBuf],
    ) -> Result<(), WatcherError> {
        info!("Watching Asset Directories: {directories:?}");
        let cwd = Utf8PathBuf::try_from(env::current_dir()?)?;
        for directory in directories.iter() {
            if self.watchers.contains_key(directory) {
                trace!("{directory} already watched");
                continue;
            }
            
            trace!("Inserting a new asset subscriber");
            {
                let cwd = cwd.clone();
                let _ = self
                    .watchers
                    .entry(directory.clone())
                    .or_try_insert_with::<WatcherError>(move || {
                        trace!("Adding watcher entry");
                        let directory = directory.clone();

                        let mut watcher = {
                            let channel = self.channel.clone();

                            notify::recommended_watcher(
                                move |file: Result<notify::Event, notify::Error>| {
                                    trace!("Got Asset Event");
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
                                        trace!("Asset Change Records: {files:?}");
                                        for file in files.iter() {
                                            let _ = channel.send(
                                                BuilderIncomingMessages::AssetChanged(
                                                    file.clone(),
                                                ),
                                            );
                                        }
                                        
                                    }
                                },
                            )?
                        };
                        trace!("Watching Directory");

                        watcher.watch(directory.as_std_path(), notify::RecursiveMode::Recursive)?;

                        trace!("Returning Watcher");

                        Ok(watcher)
                    })?;
            }
        }
        Ok(())
    }
    
    fn get_channel(&self) -> tokio::sync::broadcast::Sender<BuilderIncomingMessages> {
        self.channel.clone()
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
    use tokio::sync::broadcast::error::TryRecvError;
    use tokio::sync::broadcast::*;
    use tokio::time::timeout;

    #[tokio::test]
    async fn watcher_can_see_changes_in_a_code_directory() {
        let dir = test_temp_dir!();

        let watcher = SimpleWatcher::default();

        let mut rx = watcher.channel.subscribe();

        watcher
            .watch_code_directories(
                &[Utf8PathBuf::from_path_buf(dir.as_path_untracked().to_path_buf()).unwrap()],
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
    async fn watcher_provides_changed_files_in_asset_directory() {
        let dir = test_temp_dir!();

        let _ = File::create(dir.as_path_untracked().join("test.txt"))
        .await
        .expect("Couldn't create file");

        let watcher = SimpleWatcher::default();

        let mut rx = watcher.channel.subscribe();
        watcher
            .watch_asset_directories(
                &[Utf8PathBuf::from_path_buf(dir.as_path_untracked().to_path_buf()).unwrap()],
            )
            .expect("Couldn't set up watcher on temporary directory");


        let mut file = File::open(dir.as_path_untracked().join("test.txt"))
        .await
        .expect("Couldn't open file");

        file.write_all(b"my")
            .await
            .expect("Failed to write file");


        let result = timeout(Duration::from_millis(10), rx.recv())
            .await
            .expect("Didn't recieve initial asset message on time")
            .expect("Didn't recieve initial asset message");

        let BuilderIncomingMessages::AssetChanged(record) = result else {
            panic!("Got Message that isn't Asset Changed");
        };

        let hash = record.hash;

        assert_eq!(record.name, "test.txt");

        let mut file = File::open(dir.as_path_untracked().join("test.txt"))
            .await
            .expect("Couldn't open file");

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
