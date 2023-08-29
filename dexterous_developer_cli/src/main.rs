use clap::Parser;
use dexterous_developer_internal::HotReloadOptions;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Package to build (required in a workspace)
    #[arg(short, long)]
    package: Option<String>,

    /// Features to include
    #[arg(short, long)]
    features: Vec<String>,
}

fn main() {
    let Args { package, features } = Args::parse();

    println!("Running {package:?} with {features:?}");

    let options = HotReloadOptions {
        features,
        package,
        ..Default::default()
    };
    dexterous_developer_internal::run_reloadabe_app(options);
}
