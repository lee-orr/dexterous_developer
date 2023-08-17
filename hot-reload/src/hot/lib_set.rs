use std::{path::PathBuf, str::FromStr};

use crate::hot::HotReloadOptions;

#[derive(Debug, Clone)]
pub struct LibPathSet {
    pub folder: PathBuf,
    pub name: String,
    pub extension: String,
    pub watch_folder: PathBuf,
}

impl LibPathSet {
    pub fn new(options: &HotReloadOptions) -> Option<Self> {
        println!("Setting Up Hot Reload Paths {options:#?}");
        let HotReloadOptions {
            lib_name,
            watch_folder,
            target_folder,
        } = options;

        let root = match std::env::var("CARGO_MANIFEST_DIR") {
            Ok(v) => PathBuf::from_str(&v).ok()?,
            Err(_) => std::env::current_dir().ok()?,
        };

        println!("Root Directory {root:?}");

        let target_folder = match target_folder {
            Some(v) => {
                let mut t = root.clone();
                t.push(v);
                t
            }
            None => match std::env::current_exe() {
                Ok(mut v) => {
                    v.pop();
                    v
                }
                Err(_) => {
                    return None;
                }
            },
        };
        println!("Target Folder {target_folder:?}");

        let watch_folder = match watch_folder {
            Some(v) => {
                let mut t = root.clone();
                t.push(v);
                t
            }
            None => {
                let mut t = root.clone();
                t.push("src");
                t
            }
        };

        println!("Watch Folder {watch_folder:?}");

        let lib_name = lib_name.as_ref().cloned().unwrap_or({
            std::env::current_exe()
                .ok()
                .and_then(|v| v.file_stem().map(|v| v.to_string_lossy().to_string()))
                .map(|v| format!("lib_{v}"))?
        });

        #[cfg(unix)]
        let extension = String::from("so");
        #[cfg(windows)]
        let extension = String::from("dll");

        println!("Library Name {lib_name:?} and Extension {extension:?}");

        Some(LibPathSet {
            folder: target_folder,
            name: lib_name,
            extension,
            watch_folder,
        })
    }

    pub fn library_path(&self) -> PathBuf {
        self.folder.join(&self.name).with_extension(&self.extension)
    }
}
