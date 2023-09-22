use std::path::{Path, PathBuf};

use anyhow::{bail, Context};
use dexterous_developer_internal::HotReloadOptions;

pub(crate) fn generate_temporary_libs(
    features: &[String],
    package: Option<&str>,
    watch: &[PathBuf],
    directory: &Path,
) -> anyhow::Result<HotReloadOptions> {
    let target_folder = directory.join("target").join("dexterous");
    if !target_folder.exists() {
        std::fs::create_dir_all(&target_folder)?;
    }

    let (manifest_path, _package) = {
        let (package, lib_name, package_path, bevy, dexterous) = get_metadata(features, package)?;

        let package_path = if package_path.is_file() {
            package_path
                .parent()
                .context("Package path has no parent...")?
                .to_path_buf()
        } else {
            package_path
        };

        let temporary_lib = target_folder.join("temp_lib");
        if !temporary_lib.exists() {
            std::fs::create_dir_all(&temporary_lib)?;
        }

        let src = temporary_lib.join("src");
        if !src.exists() {
            std::fs::create_dir_all(&src)?;
        }

        let lib_file = src.join("lib.rs");
        let cargo_file = temporary_lib.join("Cargo.toml");

        let lib_file_content = format!(
            "
        #[allow(unused_imports)]
use {lib_name}::*;
        "
        );

        let cargo_file_content = format!(
            "
        [package]
name = \"game_dynamic\"
version = \"0.1.0\"
edition = \"2021\"

[dependencies]
{package} = {{ path = \"{}\" }}
bevy = {{ {bevy}, default-features = false }}
dexterous_developer = {{ {dexterous} }}

[lib]
crate-type = [\"dylib\"]

[workspace]
        ",
            dunce::canonicalize(package_path)?
                .to_string_lossy()
                .replace('\\', r"\\"),
        );

        std::fs::write(lib_file, lib_file_content)?;
        std::fs::write(&cargo_file, cargo_file_content)?;
        (dunce::canonicalize(cargo_file)?, package)
    };

    let watch_folders = if watch.is_empty() {
        let src: PathBuf = directory.join("src");
        vec![src]
    } else {
        watch.to_vec()
    };

    let options = HotReloadOptions {
        watch_folders,
        target_folder: Some(target_folder.clone()),
        package: Some("game_dynamic".to_string()),
        manifest_path: Some(manifest_path),
        ..Default::default()
    };

    Ok(options)
}

fn get_metadata(
    features: &[String],
    package: Option<&str>,
) -> anyhow::Result<(String, String, PathBuf, String, String)> {
    let mut get_metadata = cargo_metadata::MetadataCommand::new();

    get_metadata.no_deps();

    get_metadata.features(cargo_metadata::CargoOpt::SomeFeatures(features.to_vec()));

    let metadata = get_metadata.exec()?;

    let packages = metadata.packages.iter();

    let libs = packages.filter_map(|pkg| {
        if let Some(package) = package {
            let pkg = &pkg.name;
            println!("Checking package name: {package} - {pkg}");
            if pkg != package {
                return None;
            }
        }
        pkg.targets
            .iter()
            .find(|p| {
                let result = p
                    .crate_types
                    .iter()
                    .any(|v| v == "rlib" || v == "lib" || v == "dylib" || v == "cdylib");
                result
            })
            .map(|p| (pkg, p))
    });

    let libs: Vec<_> = libs.collect();

    if libs.len() > 1 {
        bail!("Workspace contains multiple libraries - please set the one you want with the --package option");
    }

    let Some((pkg, lib)) = libs.first() else {
        bail!("Workspace contains no matching libraries");
    };

    let bevy = pkg
        .dependencies
        .iter()
        .find(|v| v.name == "bevy")
        .context("Bevy is not a dependency of this package")?;
    let bevy_version = format!("version=\"{}\"", bevy.req);

    let dexterous = pkg
        .dependencies
        .iter()
        .find(|v| v.name == "dexterous_developer")
        .context("Dexterous_Developer is not a dependency of this package")?;
    let dexterous_version = if let Some(path) = &dexterous.path {
        let path = dunce::canonicalize(path.to_path_buf())?;
        format!("path=\"{}\"", path.to_string_lossy().replace('\\', r"\\"))
    } else {
        format!("version=\"{}\"", dexterous.req)
    };

    Ok((
        pkg.name.to_string(),
        lib.name.to_string(),
        pkg.manifest_path.clone().into_std_path_buf(),
        bevy_version,
        dexterous_version,
    ))
}
