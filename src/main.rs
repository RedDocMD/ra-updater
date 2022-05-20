use std::path::PathBuf;
use std::{env, process};

use octocrab::models::repos::Release;
use octocrab::Octocrab;
use thiserror::Error;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;

#[tokio::main]
async fn main() {
    let release = match latest_release().await {
        Ok(v) => v,
        Err(e) => {
            eprintln!("Failed to get latest release: {}", e);
            process::exit(1);
        }
    };
    let asset_path = match download_asset(&release).await {
        Ok(v) => v,
        Err(e) => {
            eprintln!("Failed to download asset: {}", e);
            process::exit(1);
        }
    };
}

#[derive(Debug, Error)]
enum RaUpdaterError {
    #[error("octocrab error: {0}")]
    Octocrab(#[from] octocrab::Error),

    #[error("reqwest error: {0}")]
    Reqwest(#[from] reqwest::Error),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Asset {0} not found")]
    AssetNotFound(String),
}

const RA_OWNER: &str = "rust-lang";
const RA_REPO: &str = "rust-analyzer";

async fn latest_release() -> Result<Release, RaUpdaterError> {
    let github = Octocrab::builder().build()?;
    let repo_handler = github.repos(RA_OWNER, RA_REPO);
    let latest_release = repo_handler.releases().get_latest().await?;
    Ok(latest_release)
}

const RA_ASSET_NAME: &str = "rust-analyzer-x86_64-unknown-linux-gnu.gz";

async fn download_asset(release: &Release) -> Result<PathBuf, RaUpdaterError> {
    for asset in &release.assets {
        if asset.name == RA_ASSET_NAME {
            let response = reqwest::get(asset.browser_download_url.clone()).await?;
            let mut file_path = env::temp_dir();
            file_path.push(&asset.name);
            let mut file = File::create(&file_path).await?;
            let response_bytes = response.bytes().await?;
            file.write_all(&response_bytes).await?;
            return Ok(file_path);
        }
    }
    Err(RaUpdaterError::AssetNotFound(RA_ASSET_NAME.to_string()))
}
