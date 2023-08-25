use clap::Parser;
use dexterous_developer::{internal_shared::cargo_path_utils::print_dylib_path, HotReloadOptions};

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
    println!("Search Paths");
    print_dylib_path();

    let Args { package, features } = Args::parse();

    println!("Running {package:?} with {features:?}");

    let options = HotReloadOptions {
        features,
        package,
        ..Default::default()
    };
    dexterous_developer::run_reloadabe_app(options);
}
