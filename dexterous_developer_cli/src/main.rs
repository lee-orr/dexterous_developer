use clap::Parser;

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
async fn main() {}
