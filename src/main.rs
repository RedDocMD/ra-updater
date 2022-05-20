use std::path::{Path, PathBuf};
use std::{env, process};

use flate2::bufread::GzDecoder;
use octocrab::models::repos::Release;
use octocrab::Octocrab;
use thiserror::Error;

#[tokio::main]
async fn main() {
    let latest_release = match latest_release().await {
        Ok(v) => v,
        Err(e) => {
            eprintln!("Failed to get latest release: {}", e);
            process::exit(1);
        }
    };
    let latest_version = latest_release.name.as_ref().unwrap();
    if let Some(curr_version) = curr_ra_version().await {
        if latest_version == &curr_version && ra_exists() {
            eprintln!(
                "We already have the most current version ({})",
                latest_version
            );
            return;
        }
    }

    println!("Downloading rust-analyzer {}", latest_version);
    let asset_path = match download_asset(&latest_release).await {
        Ok(v) => v,
        Err(e) => {
            eprintln!("Failed to download asset: {}", e);
            process::exit(1);
        }
    };

    println!("Deflating rust-analyzer {}", latest_version);
    if let Err(err) = deflate_asset(&asset_path) {
        eprintln!(
            "Failed to deflate rust-analyzer to correct location: {}",
            err
        );
        process::exit(1);
    }

    println!("Writing version to file");
    if let Err(err) = write_ra_version(latest_version).await {
        eprintln!("Failed to write rust-analyzer version to file: {}", err);
        process::exit(1);
    }
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

    #[error("Rust-analyzer path not obtainable")]
    RaPathNotObtainable,

    #[error("Version file path not obtainable")]
    VersionFilePathNotObtainable,
}

const RA_VERSION_FILE: &str = ".ra-version";

async fn curr_ra_version() -> Option<String> {
    use tokio::fs::File;
    use tokio::io::AsyncReadExt;

    let mut version_file_path = home::home_dir()?;
    version_file_path.push(RA_VERSION_FILE);
    let mut version_file = File::open(version_file_path).await.ok()?;
    let mut version = String::new();
    version_file.read_to_string(&mut version).await.ok()?;
    Some(version)
}

async fn write_ra_version(version: &str) -> Result<(), RaUpdaterError> {
    use tokio::fs::File;
    use tokio::io::AsyncWriteExt;

    let mut version_file_path =
        home::home_dir().ok_or(RaUpdaterError::VersionFilePathNotObtainable)?;
    version_file_path.push(RA_VERSION_FILE);
    let mut version_file = File::create(version_file_path).await?;
    version_file.write_all(version.as_bytes()).await?;

    Ok(())
}

const RA_BIN_NAME: &str = "rust-analyzer";
const RA_BIN_DIR: &str = ".local/bin";

fn ra_exists() -> bool {
    ra_path().map_or(false, |p| p.exists())
}

fn ra_path() -> Option<PathBuf> {
    let mut ra_path = home::home_dir()?;
    ra_path.push(RA_BIN_DIR);
    ra_path.push(RA_BIN_NAME);
    Some(ra_path)
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
    use tokio::fs::File;
    use tokio::io::AsyncWriteExt;

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

fn deflate_asset(asset_path: &Path) -> Result<(), RaUpdaterError> {
    use std::fs::File;
    use std::io::BufReader;
    use std::os::unix::prelude::PermissionsExt;

    let in_file = File::open(asset_path)?;
    let buf_read = BufReader::new(in_file);
    let mut gz = GzDecoder::new(buf_read);

    let out_path = ra_path().ok_or(RaUpdaterError::RaPathNotObtainable)?;
    let mut out_file = File::create(&out_path)?;
    std::io::copy(&mut gz, &mut out_file)?;

    // Set execute permission
    let mut perms = out_file.metadata()?.permissions();
    perms.set_mode(0o744);
    out_file.set_permissions(perms)?;

    Ok(())
}
