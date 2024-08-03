//! A linker that catches incremental change artifacts applies recent changes to a dynamic library
//!
//! Heavily inspired by Jon Kelley's work - <https://github.com/jkelleyrtp/ipbp/blob/main/packages/patch-linker/src/main.rs>

use camino::Utf8PathBuf;
use msvc::msvc_linker;
mod unix;
mod msvc;

pub async fn linker() -> anyhow::Result<()> {
    let mut args = std::env::args();
    args.next();
    let args = args.collect::<Vec<_>>();

    let package_name = std::env::var("DEXTEROUS_DEVELOPER_PACKAGE_NAME")?;
    let output_file = std::env::var("DEXTEROUS_DEVELOPER_OUTPUT_FILE")?;
    let target = std::env::var("DEXTEROUS_DEVELOPER_LINKER_TARGET")?;
    let lib_drectories = std::env::var("DEXTEROUS_DEVELOPER_LIB_DIRECTORES")?;
    let lib_directories: Vec<Utf8PathBuf> = serde_json::from_str(&lib_drectories)?;

    if target.contains("msvc") {
        msvc_linker(args, package_name, output_file, target, lib_directories).await
    } else {
        unix::unix_linker(args, package_name, output_file, target, lib_directories).await
    }
}