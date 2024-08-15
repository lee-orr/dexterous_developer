//! A linker that catches incremental change artifacts applies recent changes to a dynamic library
//!
//! Heavily inspired by Jon Kelley's work - <https://github.com/jkelleyrtp/ipbp/blob/main/packages/patch-linker/src/main.rs>

use camino::Utf8PathBuf;
use linker_command::LinkerCommand;

use crate::incremental_builder::builder::IncrementalRunParams;
// mod unix;
// mod msvc;
mod linker_command;

pub async fn linker() -> anyhow::Result<()> {
    let mut args = std::env::args();
    args.next();
    let args = args.collect::<Vec<_>>();

    let package_name = std::env::var("DEXTEROUS_DEVELOPER_PACKAGE_NAME")?;
    let output_file = std::env::var("DEXTEROUS_DEVELOPER_OUTPUT_FILE")?;
    let target = std::env::var("DEXTEROUS_DEVELOPER_LINKER_TARGET")?;
    let lib_drectories = std::env::var("DEXTEROUS_DEVELOPER_LIB_DIRECTORES")?;
    let lib_directories: Vec<Utf8PathBuf> = serde_json::from_str(&lib_drectories)?;
    let incremental_run_params: IncrementalRunParams =
        serde_json::from_str(&std::env::var("DEXTEROUS_DEVELOPER_INCREMENTAL_RUN")?)?;

    let mut command = LinkerCommand::new(target, args).await?;

    if command.is_main_package(&package_name) {
        eprintln!("This is a main file");
        command.add_libraries(lib_directories);
        command = command.convert_executable_to_library();
        command.output = Some(output_file.clone());

        match incremental_run_params {
            IncrementalRunParams::InitialRun => {
                let path = Utf8PathBuf::from(output_file);
                if path.exists() {
                    tokio::fs::remove_file(&path).await?;
                }
            }
            IncrementalRunParams::Patch {
                id,
                timestamp: _,
                previous_versions,
            } => {
                command = command.convert_to_patch(id, previous_versions);
            }
        }
    }

    command.execute().await?;
    Ok(())
}
