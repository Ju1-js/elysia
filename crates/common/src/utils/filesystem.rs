use std::{fs, path::Path};

/// # Errors
/// Returns an error if the directory cannot be checked or created.
pub fn ensure_dir(dir: &Path) -> Result<(), String> {
    let exists =
        fs::exists(dir).map_err(|e| format!("Cannot check if directory {} exists: {e}", dir.display()))?;
    if exists {
        return Ok(());
    }

    fs::create_dir_all(dir).map_err(|e| format!("Cannot create directory {} : {e}", dir.display()))?;

    Ok(())
}

/// # Errors
/// Returns an error if the path cannot be checked or is read-only.
pub fn ensure_writable(path: &Path) -> Result<(), String> {
    let metadata =
        fs::metadata(path).map_err(|e| format!("Cannot check if path {} exists: {e}", path.display()))?;
    if metadata.permissions().readonly() {
        return Err(format!("Path is read-only: {}", path.display()));
    }

    Ok(())
}

/// # Errors
/// Returns an error if the default directory cannot be created.
pub fn ensure_or_default<'a>(path: &'a Path, default: &Path) -> Result<&'a Path, String> {
    match ensure_dir(path) {
        Ok(()) => Ok(path),
        Err(e) => {
            println!("Error when creating/loading directory from config, will use default: {e}");
            ensure_dir(default)?;
            Ok(path)
        }
    }
}
