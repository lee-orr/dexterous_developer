use std::{env, io, path::PathBuf, process, str::FromStr};
use tracing::error;
use tracing_subscriber::{filter, prelude::*};

use clap::{Parser, Subcommand};
use dexterous_developer_types::{cargo_path_utils::add_to_dylib_path, Target};
use url::Url;

#[derive(Parser, Debug, Default)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// The working directory - this will serve as a root for any assets and other files copied over
    /// as well as being the CWD of the executed code. Defaults to the current directory.
    #[arg(short, long)]
    working_directory: Option<PathBuf>,
    /// The library directory - this is where the compiled dynamic libraries
    /// will be stored, and where they are loaded from. Defaults to "./target/reload_libs"
    #[arg(short, long)]
    library_path: Option<PathBuf>,
    /// The Url for the process handling compilation, defaults to "http://localhost:1234"
    #[arg(short, long)]
    server: Option<url::Url>,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt().pretty().init();

    let args = Args::parse();

    let working_directory = args
        .working_directory
        .or_else(|| env::current_dir().ok())
        .expect("Couldn't determine current directory");
    let library_path = args
        .library_path
        .or_else(|| {
            env::current_dir()
                .map(|dir| dir.join("target").join("reload_libs"))
                .ok()
        })
        .expect("Couldn't determine current directory");
    let server = args
        .server
        .or_else(|| url::Url::parse("http://localhost:1234").ok())
        .expect("Couldn't set up remote");

    if !working_directory.exists() {
        tokio::fs::create_dir_all(&working_directory)
            .await
            .expect("Failed to create working directory");
    }
    if !library_path.exists() {
        tokio::fs::create_dir_all(&library_path)
            .await
            .expect("Failed to create library path");
    }

    if let Err(e) = dexterous_developer_dylib_runner::run_reloadable_app(
        &working_directory,
        &library_path,
        server.clone(),
    )
    .await
    {
        match (e) {
            dexterous_developer_dylib_runner::DylibRunnerError::DylibPathsMissingLibraries => {
                let executable = env::current_exe().expect("Couldn't get current executable");
                let (env_var, env_val) = add_to_dylib_path(&library_path)
                    .expect("Failed to add library path to dylib path");
                let status = tokio::process::Command::new(executable)
                    .arg("--working-directory")
                    .arg(working_directory)
                    .arg("--library-path")
                    .arg(library_path)
                    .arg("--server")
                    .arg(server.to_string())
                    .env(env_var, env_val)
                    .status()
                    .await
                    .expect("Couldn't run with env variables");
                if let Some(code) = status.code() {
                    process::exit(code);
                } else {
                    process::exit(0);
                }
            }
            e => {
                error!("{e}");
                process::exit(1);
            }
        }
    }
    process::exit(0);
}
