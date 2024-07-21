use std::{
    collections::HashMap,
    io::{Cursor, Read},
};

use anyhow::{anyhow, bail};
use camino::Utf8PathBuf;
use dexterous_developer_types::Target;
use serde::{Deserialize, Serialize};
use tar::Archive;
use tracing::info;

pub(crate) fn zig_path() -> anyhow::Result<Utf8PathBuf> {
    let Some(project_directories) =
        directories::ProjectDirs::from("rs", "dexterous-developer", "dexterous-developer-builder")
    else {
        bail!("Can't determine download directory");
    };
    let mut base_directory =
        Utf8PathBuf::from_path_buf(project_directories.data_local_dir().to_path_buf())
            .map_err(|v| anyhow::anyhow!("Can't utf8 - {v:?}"))?;

    if !base_directory.exists() {
        std::fs::create_dir_all(&base_directory)?;
    }
    base_directory = base_directory.canonicalize_utf8()?;

    let download_directory = base_directory.join("downloader");
    let zig_directory = base_directory.join("zig");

    #[cfg(target_family = "unix")]
    let zig_path = zig_directory.join("zig");
    #[cfg(target_family = "windows")]
    let zig_path = zig_directory.join("zig.exe");

    info!("Searching for Zig at {zig_path}");

    if zig_path.exists() {
        info!("Zig found");
        return Ok(zig_path);
    }

    if download_directory.exists() {
        std::fs::remove_dir_all(&download_directory)?;
    }
    std::fs::create_dir_all(&download_directory)?;
    let download_directory = download_directory.canonicalize_utf8()?;

    if zig_directory.exists() {
        std::fs::remove_dir_all(&zig_directory)?;
    }

    info!("Set Up for Zig Download");

    let manifest = reqwest::blocking::get("https://ziglang.org/download/index.json")?.text()?;
    let manifest: HashMap<String, HashMap<String, ZigReleaseEntry>> =
        serde_json::from_str(&manifest)?;
    info!("Got Zig Manifest");

    let mut keys = manifest
        .iter()
        .filter_map(|(v, m)| semver::Version::parse(v.as_str()).ok().map(|v| (v, m)))
        .filter(|(v, _)| v.pre.is_empty())
        .collect::<Vec<_>>();
    keys.sort_by(|a, b| a.0.cmp(&b.0));
    let Some((_, latest_release)) = keys.last() else {
        bail!("Couldn't Get Latest Release");
    };

    let Some(target) = Target::current() else {
        bail!("Can't determine running platform");
    };

    let zig_target = target.zig_linker_target();

    let Some((_, ZigReleaseEntry::Artifact(artifact))) = latest_release
        .iter()
        .find(|(target, _)| zig_target.contains(*target))
    else {
        bail!("Can't find zig version");
    };

    info!("Downloading Zig Tarball {}", &artifact.tarball);

    let zig_archive = reqwest::blocking::get(&artifact.tarball)?;

    info!("Zig Downloaded");

    let content = zig_archive.bytes()?.to_vec();

    if artifact.tarball.ends_with(".zip") {
        info!("Extracting Zip to {download_directory}");
        let content = Cursor::new(content.as_slice());
        let mut archive = zip::ZipArchive::new(content)?;
        archive.extract(&download_directory)?;
    } else {
        info!("Extracting Tar to {download_directory}");
        let mut tar = xz2::read::XzDecoder::new(content.as_slice());
        let mut buf = Vec::<u8>::new();
        tar.read_to_end(&mut buf)?;
        let mut archive = Archive::new(buf.as_slice());
        archive.unpack(&download_directory)?;
    };

    info!("Extracted Tar");

    let Some(Ok(read_dir)) = std::fs::read_dir(&download_directory)?.next() else {
        bail!("No entries in download");
    };

    let path = Utf8PathBuf::from_path_buf(read_dir.path())
        .map_err(|e| anyhow!("Can't Convert {e:?} to Utf8"))?;
    if !path.as_str().contains("zig-") {
        bail!("Wrong dir");
    }

    if !path.is_dir() {
        bail!("Not a directory");
    }

    info!("Renaming {path} to {zig_directory} and removing {download_directory}");
    let output = std::process::Command::new("mv")
        .args([
            path.as_str().replace('\\', "\\\\"),
            zig_directory.as_str().replace('\\', "\\\\"),
        ])
        .output()?;
    if !output.status.success() {
        let err = std::str::from_utf8(&output.stderr).unwrap_or_default();
        bail!("Failed to copy zig directory - {} - {err}", output.status);
    }
    std::fs::remove_dir_all(&download_directory)?;

    info!("Moved from {path} to {zig_directory}");

    if !zig_path.exists() {
        bail!("Can't find zig executable at {zig_path}");
    }

    info!("Zig downloaded to {zig_path}");
    Ok(zig_path)
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct ZigArtifactInfo {
    tarball: String,
    shasum: String,
    size: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(untagged)]
enum ZigReleaseEntry {
    String(String),
    Artifact(ZigArtifactInfo),
}
