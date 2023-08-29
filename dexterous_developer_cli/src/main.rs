use std::{
    env::consts::{ARCH, OS},
    net::SocketAddr,
};

use anyhow::bail;
use axum::{
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use clap::Parser;
use dexterous_developer_internal::HotReloadOptions;
use futures_util::{future, pin_mut, StreamExt};
use tokio::process::Command;
use tower_http::services::ServeDir;

// Make our own error that wraps `anyhow::Error`.
#[derive(Debug)]
struct AppError(anyhow::Error);

// Tell axum how to convert `AppError` into a response.
impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            format!("Something went wrong: {}", self.0),
        )
            .into_response()
    }
}

// This enables using `?` on functions that return `Result<_, anyhow::Error>` to turn them into
// `Result<_, AppError>`. That way you don't need to do that manually.
impl<E> From<E> for AppError
where
    E: Into<anyhow::Error>,
{
    fn from(err: E) -> Self {
        Self(err.into())
    }
}

type Result<T> = std::result::Result<T, AppError>;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Package to build (required in a workspace)
    #[arg(short, long)]
    package: Option<String>,

    /// Features to include
    #[arg(short, long)]
    features: Vec<String>,

    /// Run as a dev server
    #[arg(short, long)]
    serve: Option<u16>,

    /// Reload from remote dev server
    /// Note - this will place the assets & hot reload files a new sub directory called "hot"
    #[arg(short, long)]
    remote: Option<url::Url>,
}

#[tokio::main]
async fn main() {
    let Args {
        package,
        features,
        serve,
        remote,
    } = Args::parse();

    if let Some(port) = serve {
        run_server(port, package, features)
            .await
            .expect("Couldn't run server");
    } else if let Some(remote) = remote {
        let targets = get_valid_targets(&remote)
            .await
            .expect("Couldn't get targets at {remote:?}");
        println!("TARGETS AVAILABLE {targets:?}");
        let target = targets.first().expect("No first target");
        println!("Connecting to {remote:?} for target{target}");
    } else {
        println!("Running {package:?} with {features:?}");

        let options = HotReloadOptions {
            features,
            package,
            ..Default::default()
        };
        dexterous_developer_internal::run_reloadabe_app(options);
    }
}

async fn run_server(port: u16, package: Option<String>, features: Vec<String>) -> Result<()> {
    let app = Router::new().route("/targets", get(list_targets));

    let asset_directory = std::env::current_dir()?.join("assets");
    let app = if asset_directory.exists() {
        app.nest_service("/assets", ServeDir::new(asset_directory))
    } else {
        app
    };

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await?;

    Ok(())
}

async fn list_targets() -> Result<Json<Vec<String>>> {
    let targets = Command::new("rustup")
        .arg("target")
        .arg("list")
        .arg("--installed")
        .output()
        .await?;
    let output = std::str::from_utf8(&targets.stdout)?
        .lines()
        .map(|v| v.to_string())
        .collect();
    Ok(Json(output))
}

async fn get_valid_targets(url: &url::Url) -> anyhow::Result<Vec<String>> {
    let result = reqwest::get(url.join("targets")?).await?;
    let json = result.json::<Vec<String>>().await?;
    let filtered: Vec<String> = json
        .into_iter()
        .filter(|v| v.contains(ARCH) && v.contains(OS))
        .collect();
    if filtered.is_empty() {
        bail!("No valid targets available at {url:?}");
    }
    Ok(filtered)
}

async fn connect_to_builds(url: &url::Url, target: String) -> anyhow::Result<()> {
    let url = url.join("connect")?.join(&target)?;
    let (ws_stream, _) = tokio_tungstenite::connect_async(url).await?;

    let (write, read) = ws_stream.split();

    todo!()
}
