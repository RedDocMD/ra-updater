use std::process;

use octocrab::models::repos::Release;
use octocrab::Octocrab;
use thiserror::Error;

#[tokio::main]
async fn main() {
    let release = match latest_release().await {
        Ok(v) => v,
        Err(e) => {
            eprintln!("Failed to get latest release: {}", e);
            process::exit(1);
        }
    };
    for asset in release.assets {
        println!("{}", asset.name);
    }
}

#[derive(Debug, Error)]
enum RaUpdaterError {
    #[error("Octocrab error: {0}")]
    Octocrab(#[from] octocrab::Error),
}

const RA_OWNER: &str = "rust-lang";
const RA_REPO: &str = "rust-analyzer";

async fn latest_release() -> Result<Release, RaUpdaterError> {
    let github = Octocrab::builder().build()?;
    let repo_handler = github.repos(RA_OWNER, RA_REPO);
    let latest_release = repo_handler.releases().get_latest().await?;
    Ok(latest_release)
}
