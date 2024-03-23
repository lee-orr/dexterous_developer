use std::{env, sync::Arc};

use camino::Utf8PathBuf;

use clap::Parser;
use dexterous_developer_builder::{simple_builder::SimpleBuilder, types::Builder};
use dexterous_developer_cli::config::DexterousConfig;
use dexterous_developer_manager::{server::run_server, Manager};
use dexterous_developer_types::PackageOrExample;

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
    let Args {
        package,
        example,
        features,
        port,
        serve_only: _,
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

    let manager = Manager::default().add_builders(&builders).await;

    run_server(port, manager).await.expect("Server Error");
}
