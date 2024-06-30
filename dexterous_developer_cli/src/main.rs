use std::{env, process, sync::Arc};

use camino::Utf8PathBuf;

use clap::Parser;
use dexterous_developer_builder::{
    simple_builder::SimpleBuilder, simple_watcher::SimpleWatcher, types::Builder,
};
use dexterous_developer_manager::{server::run_server, Manager};
use dexterous_developer_types::{config::DexterousConfig, PackageOrExample};
use tracing::info;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
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

    /// Do not run the application localy
    #[arg(short, long)]
    serve_only: bool,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt().pretty().init();

    let Args {
        package,
        example,
        features,
        port,
        serve_only,
    } = Args::parse();

    let current_directory =
        Utf8PathBuf::try_from(env::current_dir().expect("Couldn't get current directory"))
            .expect("Couldn't parse current directory");
    let config = DexterousConfig::load_toml(&current_directory)
        .await
        .expect("Couldn't load config");

    let package_or_example = match (package, example) {
        (None, None) => PackageOrExample::DefaulPackage,
        (None, Some(example)) => PackageOrExample::Example(example),
        (Some(package), None) => PackageOrExample::Package(package),
        (Some(_), Some(_)) => panic!("Can only build either a package or an example, not both"),
    };

    info!("Setting up builders for {package_or_example:?}");

    let builders = config
        .generate_build_settings(Some(package_or_example.clone()), &features)
        .expect("Failed determine build settings")
        .into_iter()
        .map(|(target, build_settings)| {
            let builder = SimpleBuilder::new(target, build_settings);
            let build: Arc<dyn Builder> = Arc::new(builder);
            build
        })
        .collect::<Vec<_>>();

    info!("Setting up Manager");

    let manager = Manager::new(Arc::new(SimpleWatcher::default()))
        .add_builders(&builders)
        .await;

    info!("Starting Server");
    if serve_only {
        run_server(port, manager).await.expect("Server Error");
    } else {
        let tempdir = async_tempfile::TempDir::new()
            .await
            .expect("Couldn't create temporary directory");
        tokio::spawn(async move {
            run_server(port, manager).await.expect("Server Error");
        });
        {
            let mut cmd = tokio::process::Command::new("dexterous_developer_runner");
            cmd.arg("--server").arg(format!("http://localhost:{port}"));
            cmd.current_dir(tempdir.dir_path());

            let mut child = cmd.spawn().expect("Couldn't execute runner");
            match child.wait().await {
                Ok(status) => {
                    if !status.success() {
                        eprintln!("Runner failed");
                        process::exit(status.code().unwrap_or_default());
                    }
                }
                Err(e) => {
                    eprintln!("Ran into an error with the runner - {e}");
                    process::exit(1);
                }
            }
        }
    }
}
