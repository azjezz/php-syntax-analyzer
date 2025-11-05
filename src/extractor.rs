use std::fs;
use std::io;
use std::path::Path;
use std::path::PathBuf;

use anyhow::Context;
use anyhow::Result;
use rayon::prelude::*;

#[tracing::instrument(name = "extracting-zip", skip(extract_to))]
fn extract_zip(zip_path: &Path, extract_to: &Path) -> Result<()> {
    let file = fs::File::open(zip_path).context("Failed to open zip file")?;
    let mut archive = zip::ZipArchive::new(file).context("Failed to read zip archive")?;

    let temp_dir = extract_to.with_extension("tmp");
    fs::create_dir_all(&temp_dir).context("Failed to create temp directory")?;

    for i in 0..archive.len() {
        let mut file = archive
            .by_index(i)
            .context("Failed to get file from archive")?;
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

    let entries: Vec<_> = fs::read_dir(&temp_dir)
        .context("Failed to read temp directory")?
        .filter_map(|e| e.ok())
        .collect();

    if entries.len() == 1 && entries[0].path().is_dir() {
        let subdir = &entries[0].path();
        fs::rename(subdir, extract_to).context("Failed to move subdirectory")?;
        fs::remove_dir(&temp_dir).context("Failed to remove temp directory")?;
    } else {
        fs::rename(&temp_dir, extract_to).context("Failed to rename temp directory")?;
    }

    Ok(())
}

#[tracing::instrument(name = "extracting-packages")]
pub fn extract_packages(target_dir: PathBuf) -> Result<usize> {
    let zipballs_dir = target_dir.join("zipballs");
    let sources_dir = target_dir.join("sources");

    fs::create_dir_all(&sources_dir).context("Failed to create sources directory")?;

    let mut zip_files = Vec::new();
    collect_zip_files(&zipballs_dir, &mut zip_files)?;

    tracing::info!("Extracting {} packages...", zip_files.len());

    let results: Vec<_> = zip_files
        .par_iter()
        .map(|zip_path| {
            let package_name = zip_path
                .strip_prefix(&zipballs_dir)
                .ok()
                .and_then(|p| p.parent())
                .and_then(|p| p.to_str())
                .unwrap_or("");

            let extract_dir = sources_dir.join(package_name);

            if extract_dir.exists() {
                tracing::debug!("Package {} already extracted, skipping", package_name);
                return Ok(());
            }

            tracing::trace!("Extracting {} to {:?}", package_name, extract_dir);
            extract_zip(zip_path, &extract_dir).with_context(|| {
                format!(
                    "Failed to extract package {} from {:?}",
                    package_name, zip_path
                )
            })
        })
        .collect();

    let mut successful = 0;
    let mut failed = 0;

    for result in results {
        match result {
            Ok(_) => successful += 1,
            Err(e) => {
                tracing::warn!("{}", e);

                failed += 1;
            }
        }
    }

    if failed > 0 {
        tracing::warn!(
            "Extraction complete: {} successful, {} failed",
            successful,
            failed
        );
    }

    Ok(successful)
}

fn collect_zip_files(dir: &Path, zip_files: &mut Vec<PathBuf>) -> Result<()> {
    if !dir.exists() {
        return Ok(());
    }

    for entry in fs::read_dir(dir).context("Failed to read directory")? {
        let entry = entry.context("Failed to read directory entry")?;
        let path = entry.path();

        if path.is_dir() {
            collect_zip_files(&path, zip_files)?;
        } else if path.extension().and_then(|s| s.to_str()) == Some("zip") {
            zip_files.push(path);
        }
    }

    Ok(())
}
