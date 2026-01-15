use std::{
    collections::HashSet,
    fs::read_dir,
    path::{Path, PathBuf},
};

use tokio::{fs::File, io::AsyncReadExt};

/// Result of a game scan with executable info
#[derive(Debug, Clone)]
pub struct GameExecutable {
    pub path: PathBuf,
    pub name: String,
    pub md5: Option<String>,
}

/// Scans a directory for executable files by filename matching
/// 
/// # Arguments
/// * `dir` - The directory to scan
/// * `max_depth` - Maximum recursion depth (0 = only current directory)
/// * `filter_names` - Optional set of specific filenames to look for. If None, returns empty results.
/// 
/// # Returns
/// Vector of paths to found executables
#[must_use]
#[allow(clippy::implicit_hasher)]
pub fn scan_for_executables(
    dir: &Path,
    max_depth: u32,
    filter_names: Option<&HashSet<String>>,
) -> Vec<PathBuf> {
    let mut results = Vec::new();
    scan_recursive(dir, filter_names, 0, max_depth, &mut results);
    results
}

/// Scans a directory for game executables with MD5 hashes
/// Matches files against provided filter names
/// 
/// # Arguments
/// * `dir` - The directory to scan
/// * `max_depth` - Maximum recursion depth (0 = only current directory)
/// * `compute_hashes` - Whether to compute MD5 hashes for found executables
/// 
/// # Returns
/// Vector of `GameExecutable` structs with paths, names, and optionally MD5 hashes
///
/// # Errors
///
/// Returns an error if:
/// - File operations fail
/// - MD5 calculation fails when `compute_hashes` is true
pub async fn scan_for_games(
    dir: &Path,
    max_depth: u32,
    compute_hashes: bool,
) -> Result<Vec<GameExecutable>, String> {
    let exe_paths = scan_for_executables(dir, max_depth, None);
    
    let mut results = Vec::new();
    for path in exe_paths {
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .map_or_else(|| format!("unknown_{}", path.display()), std::string::ToString::to_string);
        
        let md5 = if compute_hashes {
            Some(calculate_md5(&path).await?)
        } else {
            None
        };
        
        results.push(GameExecutable { path, name, md5 });
    }
    
    Ok(results)
}

fn scan_recursive(
    dir: &Path,
    filter_names: Option<&HashSet<String>>,
    current_depth: u32,
    max_depth: u32,
    out: &mut Vec<PathBuf>,
) {
    if current_depth > max_depth {
        return;
    }

    let Ok(entries) = read_dir(dir) else {
        return;
    };

    for entry in entries.into_iter().flatten() {
        let path = entry.path();
        
        if path.is_dir() {
            scan_recursive(&path, filter_names, current_depth + 1, max_depth, out);
        } else if is_executable_match(&path, filter_names) {
            out.push(path);
        }
    }
}

fn is_executable_match(path: &Path, filter_names: Option<&HashSet<String>>) -> bool {
    let Some(file_name) = path.file_name() else {
        return false;
    };
    
    let Some(name_str) = file_name.to_str() else {
        return false;
    };

    // Only match if we have a filter - return false if no filter provided
    if let Some(filter) = filter_names {
        filter.contains(name_str)
    } else {
        false
    }
}

/// Calculate MD5 hash of a file
///
/// # Errors
///
/// Returns an error if:
/// - File cannot be opened
/// - File cannot be read
pub async fn calculate_md5(path: &Path) -> Result<String, String> {
    let mut file = File::open(path)
        .await
        .map_err(|e| format!("Failed to open file: {e}"))?;
    let mut hasher = md5::Context::new();
    let mut buffer = vec![0; 8192];

    loop {
        let bytes_read = file
            .read(&mut buffer)
            .await
            .map_err(|e| format!("Failed to read file: {e}"))?;

        if bytes_read == 0 {
            break;
        }

        hasher.consume(&buffer[..bytes_read]);
    }

    let hash = hasher.finalize();
    Ok(format!("{hash:x}"))
}
