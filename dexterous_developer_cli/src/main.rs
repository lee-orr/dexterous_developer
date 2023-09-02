mod cross;
mod paths;
mod remote;
mod serve;

use std::path::PathBuf;

use clap::{Parser, Subcommand};

use dexterous_developer_internal::HotReloadOptions;
use remote::connect_to_remote;

use serve::run_server;

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
        #[arg(short = 's', long, default_value = "1234")]
        port: u16,
    },
    /// Connect to a remote dev server and run it's application locally
    Remote {
        /// Reload from remote dev server
        /// Will place all files within the current working directory, or in the reload directory
        #[arg(short, long)]
        remote: url::Url,

        /// Reload directory
        /// optional directory to use for the hot reload process
        #[arg(short, long)]
        dir: Option<PathBuf>,
    },
    ///Set up cross compilation support
    InstallCross,
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
        Commands::InstallCross => {
            println!("Setup cross compiling");
            cross::install_cross()
                .await
                .expect("Failed to install cross");
        }
    }
}
