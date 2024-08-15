use camino::Utf8PathBuf;
use std::{env, process};
use tracing::{error, info, warn};
use tracing_subscriber::{
    fmt::{self},
    layer::SubscriberExt,
    util::SubscriberInitExt,
    EnvFilter,
};

use clap::Parser;
use dexterous_developer_types::cargo_path_utils::{add_to_dylib_path, dylib_path};

#[derive(Parser, Debug, Default)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// The working directory - this will serve as a root for any assets and other files copied over
    /// as well as being the CWD of the executed code. Defaults to the current directory.
    #[arg(short, long)]
    working_directory: Option<Utf8PathBuf>,
    /// The library directory - this is where the compiled dynamic libraries
    /// will be stored, and where they are loaded from. Defaults to `./reload_libs`
    #[arg(short, long)]
    library_path: Option<Utf8PathBuf>,
    /// The Url for the process handling compilation, defaults to <http://localhost:1234>
    #[arg(short, long)]
    server: Option<url::Url>,
    /// Used to indicate that the environment variables for finding libraries should already have been set.
    /// If this is true, it will fail immediately if the libraries aren't found.
    /// Otherwise - it will try spawning a child process that sets the environment variables first.
    #[arg(long)]
    env_vars_preset: bool,
    /// Used to indicate that the library path is the target directory
    /// for builds, and the working directory is the root of the workspace
    #[arg(long)]
    in_workspace: bool,
}

fn main() {
    tracing_subscriber::registry()
        .with(fmt::layer().pretty())
        .with(EnvFilter::from_default_env())
        .init();

    let cwd =
        Utf8PathBuf::try_from(env::current_dir().expect("Couldn't determine curent directory"))
            .expect("Couldn't parse current directory");

    let args = Args::parse();

    let working_directory = args.working_directory.unwrap_or_else(|| cwd.clone());

    std::env::set_var("CARGO_MANIFEST_DIR", &working_directory);
    
    let library_path = args
        .library_path
        .unwrap_or_else(|| cwd.clone().join("reload_libs"));

    let server = args
        .server
        .or_else(|| url::Url::parse("http://localhost:1234").ok())
        .expect("Couldn't set up remote");

    info!(
        "Setting up connection to {server} in {working_directory} with libraries in {library_path}"
    );

    if !working_directory.exists() {
        std::fs::create_dir_all(&working_directory).expect("Failed to create working directory");
    }
    if !library_path.exists() {
        std::fs::create_dir_all(&library_path).expect("Failed to create library path");
    }

    if let Err(e) = dexterous_developer_dylib_runner::run_reloadable_app(
        &working_directory,
        &library_path,
        server.clone(),
        args.in_workspace,
    ) {
        match e {
            dexterous_developer_dylib_runner::error::DylibRunnerError::DylibPathsMissingLibraries => {
                if args.env_vars_preset {
                    error!("Couldn't find missing libraries");
                    error!("Library Path: {library_path}");
                    error!("Dynamic Library Path:");
                    let env = dylib_path();
                    error!("{env:?}");
                    process::exit(1);
                }
                warn!("Couldn't find library path - adding it to the environment variables and restarting");
                let executable = env::current_exe().expect("Couldn't get current executable");
                let (env_var, env_val) = (if args.in_workspace {
                    let deps = library_path.join("deps");
                    let examples = library_path.join("examples");
                    add_to_dylib_path(&[&library_path, &deps, &examples])
                } else {
                    add_to_dylib_path(&[&library_path])
                })
                    .expect("Failed to add library path to dylib path");
                let mut command = std::process::Command::new(executable);
                command.arg("--working-directory")
                .arg(working_directory)
                .arg("--library-path")
                .arg(library_path)
                .arg("--server")
                .arg(server.to_string())
                .arg("--env-vars-preset");

                if args.in_workspace {
                    command.arg("--in-workspace");
                }

                let status = command
                    .env(env_var, env_val)
                    .status()
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
