use clap::Parser;
use dexterous_developer::HotReloadOptions;

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
    println!("Loading");
    let metadata = get_metadata();

    let (package, library) = if let Some(package) = package {
        let Some(pkg) = metadata.packages.iter().find(|p| p.name == package) else {
            eprintln!("Not package named {package}");
            std::process::exit(1);
        };
        let Some(lib) = pkg.targets.iter().find(|t| {
            t.kind.contains(&"dylib".to_string()) && t.kind.contains(&"rlib".to_string())
        }) else {
            eprintln!("Package {package} has no lib target");
            std::process::exit(1);
        };
        (pkg.clone(), lib.clone())
    } else {
        let libs = metadata
            .packages
            .iter()
            .flat_map(|p| p.targets.iter().map(|t| (p.clone(), t)))
            .filter(|(_, t)| {
                t.kind.contains(&"dylib".to_string()) && t.kind.contains(&"rlib".to_string())
            })
            .collect::<Vec<_>>();
        if libs.len() > 1 {
            eprintln!("Workspace contains multiple libraries - please set the one you want with the --package option");
            std::process::exit(1);
        }
        let Some(lib) = libs.first() else {
            eprintln!("No Lib Targets Available");
            std::process::exit(1);
        };
        (lib.0.clone(), lib.1.clone())
    };

    println!("Found Package: {package:#?}");

    let mut src = library.src_path.into_std_path_buf();
    src.pop();

    let mut target_dir = metadata.target_directory.into_std_path_buf();
    target_dir.push("debug");

    println!(
        "Running with {} with target dir at {target_dir:?} with source dir {src:?}",
        library.name
    );
    let options = HotReloadOptions {
        lib_name: Some(library.name.clone()),
        watch_folder: Some(src),
        target_folder: Some(target_dir),
        features,
    };
    dexterous_developer::run_reloadabe_app(options);
}

fn get_metadata() -> cargo_metadata::Metadata {
    let mut args = std::env::args().skip_while(|val| !val.starts_with("--manifest-path"));

    let mut cmd = cargo_metadata::MetadataCommand::new();
    match args.next() {
        Some(ref p) if p == "--manifest-path" => {
            cmd.manifest_path(args.next().unwrap());
        }
        Some(p) => {
            cmd.manifest_path(p.trim_start_matches("--manifest-path="));
        }
        None => {}
    };

    let metadata = cmd.exec().unwrap();
    metadata
}
