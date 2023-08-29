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

    #[arg(short = 'm', long, default_value_t = false)]
    prefer_mold: bool,
}

fn main() {
    let Args {
        package,
        features,
        prefer_mold,
    } = Args::parse();

    println!("Running {package:?} with {features:?}");

    let options = HotReloadOptions {
        features,
        package,
        prefer_mold,
        ..Default::default()
    };
    dexterous_developer_internal::run_reloadabe_app(options);
}
