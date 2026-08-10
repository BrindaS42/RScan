use std::fs;
use std::path::{Path, PathBuf};

/// Recursively scans a directory for files, optionally filtering by extension.
///
/// Handles unreadable directories or files gracefully by skipping them.
pub fn scan_dir<P: AsRef<Path>>(dir: P, extension: Option<&str>) -> Vec<PathBuf> {
    let mut files = Vec::new();
    let norm_ext = extension.map(|e| e.trim_start_matches('.').to_lowercase());
    scan_dir_recursive(dir.as_ref(), norm_ext.as_deref(), &mut files);
    files
}

fn scan_dir_recursive(dir: &Path, extension: Option<&str>, files: &mut Vec<PathBuf>) {
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return, // Gracefully skip unreadable directories
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            scan_dir_recursive(&path, extension, files);
        } else if path.is_file() {
            if let Some(ext) = extension {
                if let Some(file_ext) = path.extension().and_then(|s| s.to_str()) {
                    if file_ext.to_lowercase() == ext {
                        files.push(path);
                    }
                }
            } else {
                files.push(path);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use tempfile::tempdir;

    #[test]
    fn test_recursive_file_collection() {
        let dir = tempdir().unwrap();
        let file1 = dir.path().join("a.txt");
        let file2 = dir.path().join("b.rs");
        File::create(&file1).unwrap();
        File::create(&file2).unwrap();

        let scanned = scan_dir(dir.path(), None);
        assert_eq!(scanned.len(), 2);
    }

    #[test]
    fn test_extension_filtering() {
        let dir = tempdir().unwrap();
        let file1 = dir.path().join("a.txt");
        let file2 = dir.path().join("b.rs");
        let file3 = dir.path().join("c.rs");
        File::create(&file1).unwrap();
        File::create(&file2).unwrap();
        File::create(&file3).unwrap();

        let scanned_rs = scan_dir(dir.path(), Some("rs"));
        assert_eq!(scanned_rs.len(), 2);

        let scanned_dot_rs = scan_dir(dir.path(), Some(".rs"));
        assert_eq!(scanned_dot_rs.len(), 2);
    }

    #[test]
    fn test_nested_directory_traversal() {
        let dir = tempdir().unwrap();
        let nested_dir = dir.path().join("a").join("b");
        fs::create_dir_all(&nested_dir).unwrap();
        let file = nested_dir.join("file.rs");
        File::create(&file).unwrap();

        let scanned = scan_dir(dir.path(), Some("rs"));
        assert_eq!(scanned.len(), 1);
        assert_eq!(scanned[0], file);
    }
}
