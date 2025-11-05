use std::borrow::Cow;
use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;

use rayon::Scope;
use rayon::prelude::*;

use mago_database::file::File;
use mago_database::file::FileType;

use crate::results::Vendor;

const PHP_EXTENSION: &[&str] = &["php", "php7", "php8"];

#[tracing::instrument(name = "reading-file", skip(sources_canonical))]
pub fn read_file(file: &Path, sources_canonical: &Path) -> Option<(Vendor, File)> {
    let bytes = fs::read(file).ok()?;
    let contents = match str::from_utf8(&bytes) {
        Ok(s) => s.to_string(),
        Err(_) => String::from_utf8_lossy(&bytes).into_owned(),
    };

    let vendor = file
        .strip_prefix(sources_canonical)
        .ok()
        .and_then(|p| {
            let mut components = p.components();
            let vendor = components.next()?.as_os_str().to_str()?;
            let package = components.next()?.as_os_str().to_str()?;
            let package_name = format!("{}/{}", vendor, package);
            Some(Vendor::from_package(&package_name))
        })
        .unwrap_or(Vendor::Other);

    Some((
        vendor,
        File::new(
            Cow::Owned(file.to_string_lossy().to_string()),
            FileType::Host,
            Some(file.to_path_buf()),
            Cow::Owned(contents),
        ),
    ))
}

#[tracing::instrument(name = "walking-files")]
pub fn walk_files(base_path: &Path) -> impl ParallelIterator<Item = PathBuf> + use<> {
    let entries = Arc::new(Mutex::new(Vec::new()));

    let base_path = base_path.to_owned();
    let move_entries = entries.clone();
    rayon::scope(move |s| s.spawn(move |s1| read_dir(move_entries, s1, base_path)));

    let entries = Arc::try_unwrap(entries).unwrap().into_inner().unwrap();
    entries.into_par_iter()
}

#[tracing::instrument(name = "reading-directory", skip(entries, s))]
fn read_dir(entries: Arc<Mutex<Vec<PathBuf>>>, s: &Scope<'_>, base_path: PathBuf) {
    for entry in fs::read_dir(base_path).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        let metadata = entry.metadata().unwrap();
        if metadata.is_dir() {
            let move_entries = entries.clone();
            s.spawn(move |s1| read_dir(move_entries, s1, path));
        } else if metadata.is_file() && has_php_extension(&path) {
            let mut locked = entries.lock().unwrap();
            locked.push(path);
        }
    }
}

fn has_php_extension(path: &Path) -> bool {
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        PHP_EXTENSION.contains(&ext)
    } else {
        false
    }
}
