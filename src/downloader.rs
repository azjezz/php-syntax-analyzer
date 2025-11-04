use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use futures::stream::{self, StreamExt};
use reqwest::Client;
use serde::Deserialize;

const PACKAGIST_PER_PAGE: usize = 15;
const MAX_CONCURRENT_DOWNLOADS: usize = 5000;

#[derive(Debug, Deserialize)]
struct PackageListResponse {
    packages: Vec<PackageItem>,
}

#[derive(Debug, Deserialize)]
struct PackageItem {
    name: String,
}

#[derive(Debug, Deserialize)]
struct PackageDetailsResponse {
    packages: HashMap<String, Vec<VersionInfo>>,
}

#[derive(Debug, Deserialize)]
struct VersionInfo {
    dist: Option<DistInfo>,
}

#[derive(Debug, Deserialize)]
struct DistInfo {
    url: String,
}

/// Fetches top packages from Packagist
async fn get_top_packages(client: &Client, min: usize, max: usize) -> Result<Vec<String>> {
    let mut packages = Vec::new();
    let mut page = (min / PACKAGIST_PER_PAGE) + 1;
    let mut collected = 0;

    tracing::info!(
        "Fetching top packages from Packagist (min: {}, max: {})",
        min,
        max
    );

    loop {
        let url = format!("https://packagist.org/explore/popular.json?page={}", page);

        tracing::debug!("Fetching page {}: {}", page, url);

        let response = client
            .get(&url)
            .send()
            .await
            .context("Failed to fetch package list")?;

        let package_list: PackageListResponse = response
            .json()
            .await
            .context("Failed to parse package list JSON")?;

        for package in package_list.packages {
            if collected >= min && collected < max {
                packages.push(package.name);
            }
            collected += 1;
            if collected >= max {
                tracing::info!("Collected {} packages", packages.len());
                return Ok(packages);
            }
        }

        page += 1;
    }
}

/// Downloads a single package
async fn download_package(client: &Client, package_name: &str, target_dir: &Path) -> Result<()> {
    let package_name_lower = package_name.to_lowercase();

    tracing::debug!("Processing package: {}", package_name);

    // Split package name into vendor and package for v2 API
    let parts: Vec<&str> = package_name_lower.split('/').collect();
    if parts.len() != 2 {
        anyhow::bail!("Invalid package name format: {}", package_name);
    }
    let (vendor, package) = (parts[0], parts[1]);

    // Fetch package metadata using v2 API
    let metadata_url = format!("https://repo.packagist.org/p2/{}/{}.json", vendor, package);

    let response = client
        .get(&metadata_url)
        .send()
        .await
        .context("Failed to fetch package metadata")?;

    let details: PackageDetailsResponse = response
        .json()
        .await
        .context("Failed to parse package metadata")?;

    let versions = details
        .packages
        .get(&package_name_lower)
        .context("Package not found in metadata")?;

    if versions.is_empty() {
        anyhow::bail!("No versions available for package");
    }

    let version_info = versions.last().context("No suitable version found")?;

    tracing::debug!("Selected version for {}", package_name);

    let dist = version_info
        .dist
        .as_ref()
        .context("No dist information available")?;

    // Create directory structure
    let zipball_dir = target_dir.join("zipballs").join(&package_name_lower);
    fs::create_dir_all(&zipball_dir).context("Failed to create zipball directory")?;

    let zipball_path = zipball_dir.join(format!("{}.zip", package_name_lower.replace('/', "-")));

    // Skip if already downloaded
    if zipball_path.exists() {
        tracing::debug!("Package {} already downloaded, skipping", package_name);
        return Ok(());
    }

    tracing::debug!("Downloading {} from {}", package_name, dist.url);

    let response = client
        .get(&dist.url)
        .send()
        .await
        .context("Failed to download package")?;

    let bytes = response
        .bytes()
        .await
        .context("Failed to read package bytes")?;

    fs::write(&zipball_path, &bytes).context("Failed to write zipball")?;

    tracing::debug!("Downloaded {} bytes to {:?}", bytes.len(), zipball_path);

    Ok(())
}

/// Downloads packages from Packagist
pub async fn download_packages(target_dir: PathBuf, min: usize, max: usize) -> Result<usize> {
    // Create necessary directories
    fs::create_dir_all(target_dir.join("zipballs"))
        .context("Failed to create zipballs directory")?;

    let client = Client::builder()
        .user_agent("php-syntax-analyzer/0.1.0")
        .build()
        .context("Failed to create HTTP client")?;

    // Fetch list of top packages
    let packages = get_top_packages(&client, min, max).await?;

    tracing::info!("Downloading {} packages...", packages.len());

    // Download packages concurrently
    let mut successful = 0;
    let mut failed = 0;

    let results: Vec<_> = stream::iter(packages)
        .map(|package_name| {
            let client = client.clone();
            let target_dir = target_dir.clone();
            async move {
                match download_package(&client, &package_name, &target_dir).await {
                    Ok(_) => Ok(()),
                    Err(e) => {
                        tracing::warn!("Failed to download {}: {}", package_name, e);
                        Err(())
                    }
                }
            }
        })
        .buffer_unordered(MAX_CONCURRENT_DOWNLOADS)
        .collect()
        .await;

    for result in results {
        match result {
            Ok(_) => successful += 1,
            Err(_) => failed += 1,
        }
    }

    if failed > 0 {
        tracing::warn!(
            "Download complete: {} successful, {} failed",
            successful,
            failed
        );
    } else {
        tracing::info!("All {} packages downloaded successfully", successful);
    }

    Ok(successful)
}
