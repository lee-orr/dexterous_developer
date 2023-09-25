use std::{
    fmt::Display,
    path::{Path, PathBuf},
};

use anyhow::{bail, Context};
use toml::{Table, Value};

pub fn setup_temporary_manifest(
    dir: &Path,
    package: Option<&str>,
) -> anyhow::Result<Option<TemporaryManifest>> {
    let mut get_metadata = cargo_metadata::MetadataCommand::new();
    get_metadata.no_deps();
    get_metadata.current_dir(dir);

    let metadata = get_metadata.exec()?;

    let packages = metadata.packages.iter();

    let libs = packages.filter_map(|pkg| {
        if let Some(package) = package {
            let pkg = pkg.name.as_str();
            if pkg != package {
                return None;
            }
        }
        pkg.targets
            .iter()
            .find(|p| {
                p.crate_types
                    .iter()
                    .map(|v| v.as_str())
                    .any(|v| v == "lib" || v == "rlib" || v == "dylib")
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

    if lib
        .crate_types
        .iter()
        .map(|v| v.as_str())
        .any(|v| v == "dylib")
    {
        return Ok(None);
    }

    let manifest_path = pkg.manifest_path.as_std_path();

    let tmp_manifest_path = manifest_path.with_extension("toml.tmp");

    std::fs::copy(manifest_path, &tmp_manifest_path)?;

    let manifest = std::fs::read_to_string(manifest_path)?;

    let mut table: Table = toml::from_str(&manifest)?;
    {
        let lib = table
            .entry("lib")
            .or_insert_with(|| Value::Table(Table::new()));

        let crate_type = lib
            .as_table_mut()
            .context("lib is not a table")?
            .entry("crate-type")
            .or_insert_with(|| Value::Array(vec![]));

        crate_type
            .as_array_mut()
            .context("crate type is not an array")?
            .insert(0, Value::String("dylib".to_string()));
    }

    let manifest = toml::to_string(&table)?;

    std::fs::write(manifest_path, manifest)?;

    Ok(Some(TemporaryManifest {
        original: tmp_manifest_path,
        temporary: manifest_path.to_path_buf(),
    }))
}

pub struct TemporaryManifest {
    original: PathBuf,
    temporary: PathBuf,
}

impl Drop for TemporaryManifest {
    fn drop(&mut self) {
        println!("Clearing Temporary Manifest");
        if let Err(e) = std::fs::remove_file(&self.temporary) {
            eprintln!("Couldn't remove temporary file - {e:?}");
        }
        if let Err(e) = std::fs::rename(&self.original, &self.temporary) {
            eprintln!("Couldn't reset original manifest {e:?}");
        }
    }
}

impl Display for TemporaryManifest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.original.to_string_lossy())
    }
}
