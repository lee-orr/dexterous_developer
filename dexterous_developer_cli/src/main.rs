mod remote;
mod serve;

use clap::Parser;

use dexterous_developer_internal::HotReloadOptions;
use remote::connect_to_remote;

use serve::run_server;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Package to build (required in a workspace)
    #[arg(short, long)]
    package: Option<String>,

    /// Features to include
    #[arg(short, long)]
    features: Vec<String>,

    /// Prefer the Mold linker on Linux
    #[arg(short = 'm', long, default_value_t = false)]
    prefer_mold: bool,

    /// Run as a dev server
    #[arg(short, long)]
    serve: Option<u16>,

    /// Reload from remote dev server
    /// Will place all files within the current working directory, or in the reload directory
    #[arg(short, long)]
    remote: Option<url::Url>,

    /// Reload Directory
    #[arg(short = 'd', long)]
    reload_dir: Option<std::path::PathBuf>,
}

#[tokio::main]
async fn main() {
    let Args {
        package,
        features,
        prefer_mold,
        serve,
        remote,
        reload_dir,
    } = Args::parse();

    if let Some(port) = serve {
        run_server(port, package, features, prefer_mold)
            .await
            .expect("Couldn't run server");
    } else if let Some(remote) = remote {
        connect_to_remote(remote, reload_dir)
            .await
            .expect("Remote Connection Failed");
    } else {
        println!("Running {package:?} with {features:?}");

        let options = HotReloadOptions {
            features,
            package,
            prefer_mold,
            ..Default::default()
        };
        dexterous_developer_internal::run_reloadabe_app(options);
    }
}
