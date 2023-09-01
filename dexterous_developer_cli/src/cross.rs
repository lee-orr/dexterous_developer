use std::{
    collections::HashMap,
    env::consts::{ARCH, OS},
    path::{Path, PathBuf},
};

use anyhow::{bail, Context};
use directories::ProjectDirs;
use futures_util::future::join_all;
use semver::{Version, VersionReq};
use serde::Deserialize;
use url::Url;

use crate::paths::{get_paths, CliPaths};

pub async fn install_cross() -> anyhow::Result<()> {
    let CliPaths { data, ziglang } = get_paths()?;

    println!("downloading zig manifest");
    let get_zig = reqwest::get("https://ziglang.org/download/index.json")
        .await
        .context("trying to get the zig manifest")?;
    let zig_manifest = get_zig
        .json::<ZigManifest>()
        .await
        .context("Parsing zig manifest")?;

    let latest = zig_manifest
        .0
        .iter()
        .fold(None, |latest, version| {
            if version.0.as_str() == "master" {
                return latest;
            }
            let Ok(b) = Version::parse(version.0) else {
                return latest;
            };
            if let Some((a, _)) = &latest {
                if a > &b {
                    latest
                } else {
                    Some((b, version.1))
                }
            } else {
                Some((b, version.1))
            }
        })
        .context("Couldn't find a zig version")?;

    println!("Latest version is {}", latest.0);
    println!(
        "Looking at keys {:?}",
        latest.1.keys().collect::<Vec<&String>>()
    );

    let target = format!("{ARCH}-{OS}");

    println!("Current target: {target}");

    let download_path = latest.1.get(&target).context("Couldn't find zig versom")?;

    let download_path = match download_path {
        ZigManifestListing::Value(_) => bail!("Invalid Manifest Listing"),
        ZigManifestListing::File { tarball } => Url::parse(tarball.as_str())?,
    };

    println!("Downloading zig from {download_path}");
    let tar_path =
        data.join("ziglang")
            .with_extension(if download_path.to_string().ends_with("zip") {
                "zip"
            } else {
                "tar.xz"
            });

    if tar_path.exists() {
        tokio::fs::remove_file(tar_path.as_path()).await?;
    }

    let response = reqwest::get(download_path).await?;
    if !response.status().is_success() {
        bail!("Download failed - {response:?}");
    }
    let content = response.bytes().await?;
    println!("Downloading to {tar_path:?}");
    tokio::fs::write(tar_path.as_path(), content)
        .await
        .context("Write to {target:?} failed")?;

    let unzip = tokio::process::Command::new("tar")
        .current_dir(data.as_path())
        .arg("-xf")
        .arg(tar_path.as_path())
        .status()
        .await?;

    if !unzip.success() {
        bail!("Failed to unzip zig");
    }

    tokio::fs::remove_file(tar_path).await?;

    let tmp_zig_folder = data.join(format!("zig-{OS}-{ARCH}-{}", latest.0));

    if !tmp_zig_folder.exists() {
        bail!("Temporary Zig Folder Doesn't Exist");
    }

    if ziglang.exists() {
        tokio::fs::remove_dir_all(ziglang.as_path()).await?;
    }

    tokio::fs::rename(tmp_zig_folder.as_path(), ziglang.as_path()).await?;

    let ziglang = ziglang.to_string_lossy();

    for (rust, zig) in CROSS_TARGETS.iter() {
        setup_target(*rust, *zig, data.as_path(), &ziglang).await?;
    }

    Ok(())
}

#[derive(Deserialize)]
struct ZigManifest(HashMap<String, HashMap<String, ZigManifestListing>>);

#[derive(Deserialize)]
#[serde(untagged)]
enum ZigManifestListing {
    Value(String),
    File { tarball: String },
}

pub fn generate_zig_path_for_target(target: &str, data: &Path) -> PathBuf {
    data.join(format!("zig-{target}.sh"))
}

async fn setup_target(rust: &str, zig: &str, data: &Path, ziglang: &str) -> anyhow::Result<()> {
    println!("Checking for target: {rust}");
    let output = tokio::process::Command::new("rustup")
        .arg("+nightly")
        .arg("target")
        .arg("list")
        .arg("--installed")
        .output()
        .await?;
    let std_out = std::str::from_utf8(&output.stdout)?;
    if !std_out.contains(rust) {
        println!("Installing target: {rust}");
        let result = tokio::process::Command::new("rustup")
            .arg("+nightly")
            .arg("target")
            .arg("add")
            .arg(rust)
            .status()
            .await;
        match result {
            Ok(result) => {
                if !result.success() {
                    eprintln!("Failed to install {rust}");
                    eprintln!("{result:?}");
                    bail!("Failed to install {rust} with {result:?}");
                } else {
                    println!("Successfully installed {rust}");
                }
            }
            Err(e) => {
                eprintln!("Failed to install {rust} with error: {e:?}");
                bail!("Failed to install {rust} with error: {e:?}")
            }
        }
    } else {
        println!("{rust} already installed");
    }

    let path = generate_zig_path_for_target(rust, data);

    let contents = format!(
        r#"#!/bin/sh
{ziglang}/zig cc --target {zig} $@
"#
    );

    tokio::fs::write(path, contents).await?;

    Ok(())
}

pub const CROSS_TARGETS: &[(&str, &str)] = &[
    ("aarch64-unknown-linux-gnu", "aarch64-linux"),
    ("x86_64-unknown-linux-gnu", "x86_64-linux"),
    ("x86_64-pc-windows-gnu", "x86_64-windows"),
    ("aarch64-apple-darwin", "aarch64-macos"),
    ("x86_64-apple-darwin", "x86_64-macos"),
];
