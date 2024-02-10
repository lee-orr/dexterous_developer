use std::path::PathBuf;

use clap::{Parser, Subcommand};

use dexterous_developer_internal::{compile_reloadable_libraries, HotReloadOptions, Target};
use url::Url;

use dexterous_developer_cli::{
    cross::AppleSDKPath,
    cross::{self, check_cross_requirements_installed},
    existing::load_existing_directory,
    paths::{self, CliPaths},
    remote::connect_to_remote,
    serve::run_server,
    temporary_manifest::setup_temporary_manifest,
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

        /// Example to build
        #[arg(short, long)]
        example: Option<String>,

        /// Features to include
        #[arg(short, long)]
        features: Vec<String>,

        /// Folders to watch - defaults to src in the package root
        #[arg(short, long)]
        watch: Vec<PathBuf>,
    },
    /// Start a dev server for remote, hot reloaded development
    Serve {
        /// Package to build (required in a workspace)
        #[arg(short, long)]
        package: Option<String>,

        /// Example to build
        #[arg(short, long)]
        example: Option<String>,

        /// Features to include
        #[arg(short, long)]
        features: Vec<String>,

        /// Port to host on
        #[arg(default_value = "1234")]
        port: u16,

        /// Folders to watch - defaults to src in the package root
        #[arg(short, long)]
        watch: Vec<PathBuf>,
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
        // /// Macos SDK Tarball File Path
        // #[arg(long)]
        // macos_sdk_url: Option<PathBuf>,
        #[arg(long)]
        macos_sdk_url: Option<Url>,
        // /// iOS SDK Tarball File Path
        // #[arg(long)]
        // ios_sdk_url: Option<PathBuf>,
        #[arg(long)]
        ios_sdk_url: Option<Url>,

        /// The targets you want to install. Options are: linux, linux-arm, windows, mac, mac-arm
        #[arg(required = true)]
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

        /// Example to build
        #[arg(short, long)]
        example: Option<String>,

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
            example: None,
            features: vec![],
            watch: vec![],
        }
    }
}

#[tokio::main]
async fn main() {
    if std::env::var("DEXTEROUS_BUILD_SETTINGS").is_ok() {
        let result = dexterous_developer_internal::run_reloadabe_app(Default::default());
        if let Err(e) = result {
            eprintln!("Run Failed with error - {e}");
            std::process::exit(1);
        }
        std::process::exit(0);
    }

    let Args { command } = Args::parse();
    let dir = std::env::current_dir().expect("No current directory - nothing to run");
    println!("Current directory: {:?}", &dir);
    std::env::set_var("CARGO_MANIFEST_DIR", &dir);

    match command {
        Commands::Run {
            package,
            example,
            features,
            watch,
        } => {
            println!("Running {package:?} with {features:?}");

            let temporary = setup_temporary_manifest(&dir, package.as_deref(), example.as_deref())
                .expect("Couldn't set up temporary manifest");

            let options = HotReloadOptions {
                features,
                package,
                example,
                watch_folders: watch,
                ..Default::default()
            };

            let result = dexterous_developer_internal::run_reloadabe_app(options);
            if let Some(manifest) = temporary {
                println!("Resetting original manifest - {manifest}");
            };
            if let Err(e) = result {
                eprintln!("Run Failed with error - {e}");
                std::process::exit(1);
            }
        }
        Commands::Serve {
            package,
            example,
            features,
            watch,
            port,
        } => {
            println!("Serving {package:?} on port {port}");

            let temporary = setup_temporary_manifest(&dir, package.as_deref(), example.as_deref())
                .expect("Couldn't set up temporary manifest");
            run_server(port, package, features, watch)
                .await
                .expect("Couldn't run server");
            if let Some(manifest) = temporary {
                println!("Resetting original manifest - {manifest}");
            };
        }
        Commands::Remote { remote, dir } => {
            connect_to_remote(remote, dir)
                .await
                .expect("Remote Connection Failed");
        }
        Commands::InstallCross {
            macos_sdk_url,
            ios_sdk_url,
            targets,
        } => {
            let macos_sdk = macos_sdk_url.map(AppleSDKPath::Url);
            let ios_sdk_url = ios_sdk_url.map(AppleSDKPath::Url);
            println!("Setup cross compiling");
            cross::install_cross(&targets, macos_sdk, ios_sdk_url)
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
            example,
            features,
            libs,
            target,
        } => {
            let temporary = setup_temporary_manifest(&dir, package.as_deref(), example.as_deref())
                .expect("Couldn't set up temporary manifest");
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
            if let Some(manifest) = temporary {
                println!("Resetting original manifest - {manifest}");
            };
        }
    }
}
