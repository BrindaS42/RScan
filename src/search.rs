use rayon::prelude::*;
use std::path::{Path, PathBuf};

/// Represents a single matching line in a file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchResult {
    pub path: PathBuf,
    pub line_number: usize,
    pub line_content: String,
}

/// Searches a single file for a given text pattern.
///
/// Returns `Ok(Vec<SearchResult>)` if the file was read successfully, or `Err(io::Error)`
/// if an I/O error occurred (e.g. unreadable file or invalid UTF-8 binary data).
pub fn search_file(path: &Path, pattern: &str) -> std::io::Result<Vec<SearchResult>> {
    let content = std::fs::read_to_string(path)?;
    if !content.contains(pattern) {
        return Ok(Vec::new());
    }

    let mut matches = Vec::new();
    for (idx, line) in content.lines().enumerate() {
        if line.contains(pattern) {
            matches.push(SearchResult {
                path: path.to_path_buf(),
                line_number: idx + 1,
                line_content: line.to_string(),
            });
        }
    }

    Ok(matches)
}

/// Sequentially searches multiple files for a pattern, gracefully skipping unreadable files.
pub fn search_sequential(files: &[PathBuf], pattern: &str) -> Vec<SearchResult> {
    let mut results = Vec::new();
    for file in files {
        if let Ok(file_matches) = search_file(file, pattern) {
            results.extend(file_matches);
        }
    }
    results
}

/// Parallely searches multiple files using Rayon, gracefully skipping unreadable files.
pub fn search_parallel(files: &[PathBuf], pattern: &str) -> Vec<SearchResult> {
    files
        .par_iter()
        .filter_map(|file| search_file(file, pattern).ok())
        .flat_map(|matches| matches)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn test_search_file_single_match() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.txt");
        let mut file = File::create(&file_path).unwrap();
        writeln!(file, "Hello world\nTODO: fix item\nGoodbye").unwrap();

        let results = search_file(&file_path, "TODO").unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].line_number, 2);
        assert_eq!(results[0].line_content, "TODO: fix item");
    }

    #[test]
    fn test_search_file_no_match() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.txt");
        let mut file = File::create(&file_path).unwrap();
        writeln!(file, "Hello world\nGoodbye").unwrap();

        let results = search_file(&file_path, "TODO").unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn test_search_file_multiple_matches() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.txt");
        let mut file = File::create(&file_path).unwrap();
        writeln!(file, "TODO: item 1\nsome code\nTODO: item 2").unwrap();

        let results = search_file(&file_path, "TODO").unwrap();
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].line_number, 1);
        assert_eq!(results[1].line_number, 3);
    }

    #[test]
    fn test_search_empty_file() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("empty.txt");
        File::create(&file_path).unwrap();

        let results = search_file(&file_path, "pattern").unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn test_sequential_and_parallel_search_equivalence() {
        let dir = tempdir().unwrap();
        let file1 = dir.path().join("f1.txt");
        let file2 = dir.path().join("f2.txt");

        let mut f1 = File::create(&file1).unwrap();
        writeln!(f1, "Line 1\nTODO: refactor f1\nLine 3").unwrap();

        let mut f2 = File::create(&file2).unwrap();
        writeln!(f2, "TODO: start f2\nLine 2").unwrap();

        let files = vec![file1, file2];

        let seq_results = search_sequential(&files, "TODO");
        let par_results = search_parallel(&files, "TODO");

        assert_eq!(seq_results.len(), 2);
        assert_eq!(seq_results.len(), par_results.len());
    }
}
