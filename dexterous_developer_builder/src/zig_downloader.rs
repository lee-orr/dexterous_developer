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

pub(crate) async fn zig_path() -> anyhow::Result<Utf8PathBuf> {
    let Some(project_directories) =
        directories::ProjectDirs::from("rs", "dexterous-developer", "dexterous-developer-builder")
    else {
        bail!("Can't determine download directory");
    };
    let mut base_directory = Utf8PathBuf::from_path_buf(project_directories.data_local_dir().to_path_buf())
        .map_err(|v| anyhow::anyhow!("Can't utf8 - {v:?}"))?;

    if !base_directory.exists() {
        base_directory = base_directory.canonicalize_utf8()?;
    }

    let download_directory = base_directory.join("downloader");
    let zig_directory = base_directory.join("zig");
    let zig_path = zig_directory.join("zig");

    info!("Searching for Zig at {zig_path}");

    if zig_path.exists() {
        info!("Zig found");
        return Ok(zig_path);
    }

    if download_directory.exists() {
        tokio::fs::remove_dir_all(&download_directory).await?;
    }
    tokio::fs::create_dir_all(&download_directory).await?;
    let download_directory =  download_directory.canonicalize_utf8()?;

    if zig_directory.exists() {
        tokio::fs::remove_dir_all(&zig_directory).await?;
    }

    info!("Set Up for Zig Download");

    let manifest = reqwest::get("https://ziglang.org/download/index.json")
        .await?
        .text()
        .await?;
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

    let zig_archive = reqwest::get(&artifact.tarball).await?;

    info!("Zig Downloaded");

    let content = zig_archive.bytes().await?.to_vec();

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

    let Some(read_dir) = tokio::fs::read_dir(&download_directory)
        .await?
        .next_entry()
        .await?
    else {
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
    let output = tokio::process::Command::new("cp").arg("-r").args([&path, &zig_directory]).output().await?;
    if !output.status.success() {
        bail!("Failed to copy zig directory - {}", output.status);
    }
    tokio::fs::remove_dir_all(&download_directory).await?;

    info!("Moved from {path} to {zig_directory}");

    if !zig_path.exists() {
        bail!("Can't find zig executable");
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
