use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::fs;
use std::io;

use anyhow::{Context, Result};
use futures::stream::{self, StreamExt};
use reqwest::Client;
use serde::Deserialize;

const PACKAGIST_PER_PAGE: usize = 15;
const MAX_CONCURRENT_DOWNLOADS: usize = 5;

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

    tracing::info!("Fetching top packages from Packagist (min: {}, max: {})", min, max);

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

/// Downloads and extracts a single package
async fn download_and_extract_package(
    client: &Client,
    package_name: &str,
    target_dir: &Path,
) -> Result<()> {
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

    // Get versions for this package
    let versions = details
        .packages
        .get(&package_name_lower)
        .context("Package not found in metadata")?;

    if versions.is_empty() {
        anyhow::bail!("No versions available for package");
    }

    // Pick version: just pick the last version in the array
    let version_info = versions
        .last()
        .context("No suitable version found")?;

    tracing::debug!("Selected version for {}", package_name);

    let dist = version_info.dist.as_ref().context("No dist information available")?;

    // Create directory structure
    let zipball_dir = target_dir.join("zipballs").join(&package_name_lower);
    fs::create_dir_all(&zipball_dir).context("Failed to create zipball directory")?;

    let zipball_path = zipball_dir.join(format!("{}.zip", package_name_lower.replace('/', "-")));

    // Skip if already downloaded
    if zipball_path.exists() {
        tracing::debug!("Package {} already downloaded, skipping", package_name);
    } else {
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
    }

    // Extract the package
    let extract_dir = target_dir.join("sources").join(&package_name_lower);

    if extract_dir.exists() {
        tracing::debug!("Package {} already extracted, skipping", package_name);
    } else {
        tracing::trace!("Extracting {} to {:?}", package_name, extract_dir);
        extract_zip(&zipball_path, &extract_dir).context("Failed to extract package")?;
    }

    Ok(())
}

/// Extracts a zip file and flattens the directory structure
fn extract_zip(zip_path: &Path, extract_to: &Path) -> Result<()> {
    let file = fs::File::open(zip_path).context("Failed to open zip file")?;
    let mut archive = zip::ZipArchive::new(file).context("Failed to read zip archive")?;

    // Create a temporary extraction directory
    let temp_dir = extract_to.with_extension("tmp");
    fs::create_dir_all(&temp_dir).context("Failed to create temp directory")?;

    // Extract all files
    for i in 0..archive.len() {
        let mut file = archive.by_index(i).context("Failed to get file from archive")?;
        let outpath = temp_dir.join(file.name());

        if file.is_dir() {
            fs::create_dir_all(&outpath).context("Failed to create directory")?;
        } else {
            if let Some(parent) = outpath.parent() {
                fs::create_dir_all(parent).context("Failed to create parent directory")?;
            }
            let mut outfile = fs::File::create(&outpath).context("Failed to create output file")?;
            io::copy(&mut file, &mut outfile).context("Failed to copy file contents")?;
        }
    }

    // Find the subdirectory (packages are usually extracted into a single root directory)
    let entries: Vec<_> = fs::read_dir(&temp_dir)
        .context("Failed to read temp directory")?
        .filter_map(|e| e.ok())
        .collect();

    if entries.len() == 1 && entries[0].path().is_dir() {
        // Move contents from subdirectory to final location
        let subdir = &entries[0].path();
        fs::rename(subdir, extract_to).context("Failed to move subdirectory")?;
        fs::remove_dir(&temp_dir).context("Failed to remove temp directory")?;
    } else {
        // No subdirectory, just rename temp to final location
        fs::rename(&temp_dir, extract_to).context("Failed to rename temp directory")?;
    }

    Ok(())
}

/// Downloads and extracts packages from Packagist
pub async fn download_and_extract_packages(target_dir: PathBuf, min: usize, max: usize) -> Result<usize> {
    // Create necessary directories
    fs::create_dir_all(target_dir.join("zipballs"))
        .context("Failed to create zipballs directory")?;
    fs::create_dir_all(target_dir.join("sources"))
        .context("Failed to create sources directory")?;

    let client = Client::builder()
        .user_agent("php-syntax-analyzer/0.1.0")
        .build()
        .context("Failed to create HTTP client")?;

    // Fetch list of top packages
    let packages = get_top_packages(&client, min, max).await?;

    // Download and extract packages concurrently
    let mut successful = 0;
    let mut _failed = 0;

    let results: Vec<_> = stream::iter(packages)
        .map(|package_name| {
            let client = client.clone();
            let target_dir = target_dir.clone();
            async move {
                match download_and_extract_package(&client, &package_name, &target_dir).await {
                    Ok(_) => Ok(()),
                    Err(e) => {
                        tracing::warn!("Failed to process {}: {}", package_name, e);
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
            Err(_) => _failed += 1,
        }
    }

    Ok(successful)
}
