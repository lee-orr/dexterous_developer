mod cross;
mod existing;
mod paths;
mod remote;
mod serve;

use std::path::PathBuf;

use clap::{Parser, Subcommand};

use dexterous_developer_internal::{compile_reloadable_libraries, HotReloadOptions, Target};
use existing::load_existing_directory;
use remote::connect_to_remote;

use serve::run_server;
use url::Url;

use crate::{
    cross::{check_cross_requirements_installed, AppleSDKPath},
    paths::CliPaths,
};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Run and launch a hot reloaded application
    Run {
        /// Package to build (required in a workspace)
        #[arg(short, long)]
        package: Option<String>,

        /// Features to include
        #[arg(short, long)]
        features: Vec<String>,
    },
    /// Start a dev server for remote, hot reloaded development
    Serve {
        /// Package to build (required in a workspace)
        #[arg(short, long)]
        package: Option<String>,

        /// Features to include
        #[arg(short, long)]
        features: Vec<String>,

        /// Port to host on
        #[arg(default_value = "1234")]
        port: u16,
    },
    /// Connect to a remote dev server and run it's application locally
    Remote {
        /// Reload from remote dev server
        /// Will place all files within the current working directory, or in the reload directory
        remote: url::Url,

        /// Reload directory
        /// optional directory to use for the hot reload process
        #[arg(short, long)]
        dir: Option<PathBuf>,
    },
    /// Set up cross compilation support
    InstallCross {
        /// Macos SDK Tarball File Path
        #[arg(long)]
        macos_sdk_file: Option<PathBuf>,

        /// Macos SDK Tarball Download URL - only used if macos_sdk_dir is not provided
        #[arg(long)]
        macos_sdk_url: Option<Url>,

        targets: Vec<Target>,
    },
    /// Run a pre-existing set of compiled libraries. Mostly useful for debugging purposes.
    RunExisting {
        /// The location of the existing libraries
        #[arg(default_value = "./libs")]
        libs: PathBuf,
    },
    /// Compile reloading libraries, without running them. Mostly useful for debugging purposes.
    CompileLibs {
        /// Package to build (required in a workspace)
        #[arg(short, long)]
        package: Option<String>,

        /// Features to include
        #[arg(short, long)]
        features: Vec<String>,

        /// Target
        #[arg(short, long)]
        target: Option<String>,

        /// The location of the existing libraries
        #[arg(default_value = "./libs")]
        libs: PathBuf,
    },
}

impl Default for Commands {
    fn default() -> Self {
        Self::Run {
            package: None,
            features: vec![],
        }
    }
}

#[tokio::main]
async fn main() {
    if std::env::var("DEXTEROUS_BUILD_SETTINGS").is_ok() {
        dexterous_developer_internal::run_reloadabe_app(Default::default());
    }

    let Args { command } = Args::parse();

    match command {
        Commands::Run { package, features } => {
            println!("Running {package:?} with {features:?}");

            let options = HotReloadOptions {
                features,
                package,
                ..Default::default()
            };
            dexterous_developer_internal::run_reloadabe_app(options);
        }
        Commands::Serve {
            package,
            features,
            port,
        } => {
            println!("Serving {package:?} on port {port}");
            run_server(port, package, features)
                .await
                .expect("Couldn't run server");
        }
        Commands::Remote { remote, dir } => {
            connect_to_remote(remote, dir)
                .await
                .expect("Remote Connection Failed");
        }
        Commands::InstallCross {
            macos_sdk_url,
            macos_sdk_file,
            targets,
        } => {
            let macos_sdk = macos_sdk_url.map(AppleSDKPath::Url);
            println!("Setup cross compiling");
            cross::install_cross(&targets, macos_sdk)
                .await
                .expect("Failed to install cross");
        }
        Commands::RunExisting { libs } => {
            println!("Running existing libraries");
            load_existing_directory(libs)
                .await
                .expect("Couldn't run existing libs");
        }
        Commands::CompileLibs {
            package,
            features,
            libs,
            target,
        } => {
            let CliPaths { cross_config, .. } = paths::get_paths().expect("Couldn't get cli paths");
            if cross_config.exists() {
                std::env::set_var("CROSS_CONFIG", &cross_config);
            }
            let target = target.map(|v| v.parse::<Target>().expect("Invalid Target {v}"));

            println!("Compiling Reloadable Libs");

            if let Some(target) = target.as_ref() {
                check_cross_requirements_installed(target)
                    .expect("Cross Compilation Requirements Missing");
            }

            let options = HotReloadOptions {
                package,
                features,
                build_target: target,
                ..Default::default()
            };

            compile_reloadable_libraries(options, &libs)
                .expect("Couldn't compile reloadable library");
        }
    }
}
