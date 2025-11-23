use std::path::Path;
use walkdir::WalkDir;
#[cfg(not(windows))]
use rayon::prelude::*;
use crate::error::{Result, RsyncError};
use crate::filesystem::file_info::FileInfo;
use crate::filesystem::path_utils::{normalize_path, to_long_path, exceeds_max_path};


pub struct Scanner {

    pub recursive: bool,


    pub follow_symlinks: bool,


    #[allow(dead_code)]
    pub parallel: bool,
}

impl Default for Scanner {
    fn default() -> Self {
        Self {
            recursive: true,
            follow_symlinks: false,
            parallel: true,
        }
    }
}

impl Scanner {
    pub fn new() -> Self {
        Self::default()
    }


    pub fn recursive(mut self, recursive: bool) -> Self {
        self.recursive = recursive;
        self
    }


    pub fn follow_symlinks(mut self, follow: bool) -> Self {
        self.follow_symlinks = follow;
        self
    }


    #[allow(dead_code)]
    pub fn parallel(mut self, parallel: bool) -> Self {
        self.parallel = parallel;
        self
    }


    pub fn scan(&self, path: &Path) -> Result<Vec<FileInfo>> {

        let normalized = if path.exists() {
            normalize_path(path)?
        } else {
            path.to_path_buf()
        };


        let scan_path = if exceeds_max_path(&normalized) {
            to_long_path(&normalized)?
        } else {
            normalized
        };


        if !scan_path.exists() {
            return Err(RsyncError::InvalidPath(path.to_path_buf()));
        }


        if scan_path.is_file() {
            let metadata = std::fs::metadata(&scan_path)
                .map_err(|e| RsyncError::Io(e))?;
            return Ok(vec![FileInfo::from_metadata(scan_path, &metadata)]);
        }


        if !self.recursive {

            return self.scan_directory_non_recursive(&scan_path);
        }


        self.scan_directory_recursive(&scan_path)
    }


    fn scan_directory_non_recursive(&self, path: &Path) -> Result<Vec<FileInfo>> {

        #[cfg(windows)]
        {
            use crate::filesystem::windows_scanner::WindowsScanner;
            let scanner = WindowsScanner::new()
                .recursive(false)
                .follow_symlinks(self.follow_symlinks);
            return scanner.scan(path);
        }


        #[cfg(not(windows))]
        {
            let mut files = Vec::new();

            let entries = std::fs::read_dir(path)
                .map_err(|e| RsyncError::Io(e))?;

            for entry in entries {
                let entry = entry.map_err(|e| RsyncError::Io(e))?;
                let entry_path = entry.path();

                let metadata = if self.follow_symlinks {
                    std::fs::metadata(&entry_path)
                } else {
                    std::fs::symlink_metadata(&entry_path)
                }.map_err(|e| RsyncError::Io(e))?;

                files.push(FileInfo::from_metadata(entry_path, &metadata));
            }

            Ok(files)
        }
    }


    fn scan_directory_recursive(&self, path: &Path) -> Result<Vec<FileInfo>> {

        #[cfg(windows)]
        {
            use crate::filesystem::windows_scanner::WindowsScanner;
            let scanner = WindowsScanner::new()
                .recursive(true)
                .follow_symlinks(self.follow_symlinks);
            return scanner.scan(path);
        }


        #[cfg(not(windows))]
        {
            let walker = WalkDir::new(path)
                .follow_links(self.follow_symlinks)
                .into_iter()
                .filter_map(|e| e.ok());

            if self.parallel {

                let entries: Vec<_> = walker.collect();

                let files: Result<Vec<FileInfo>> = entries
                    .par_iter()
                    .map(|entry| {
                        let metadata = if self.follow_symlinks {
                            entry.metadata().map_err(|e| RsyncError::Io(std::io::Error::from(e)))?
                        } else {
                            entry.path().symlink_metadata().map_err(|e| RsyncError::Io(e))?
                        };

                        Ok(FileInfo::from_metadata(entry.path().to_path_buf(), &metadata))
                    })
                    .collect();

                files
            } else {

                let mut files = Vec::new();

                for entry in walker {
                    let metadata = if self.follow_symlinks {
                        entry.metadata().map_err(|e| RsyncError::Io(std::io::Error::from(e)))?
                    } else {
                        entry.path().symlink_metadata().map_err(|e| RsyncError::Io(e))?
                    };

                    files.push(FileInfo::from_metadata(entry.path().to_path_buf(), &metadata));
                }

                Ok(files)
            }
        }
    }


    #[allow(dead_code)]
    pub fn count_files(&self, path: &Path) -> Result<usize> {
        let scan_path = if exceeds_max_path(path) {
            to_long_path(path)?
        } else {
            path.to_path_buf()
        };

        if !scan_path.exists() {
            return Err(RsyncError::InvalidPath(path.to_path_buf()));
        }

        if scan_path.is_file() {
            return Ok(1);
        }

        let count = WalkDir::new(&scan_path)
            .follow_links(self.follow_symlinks)
            .into_iter()
            .filter_map(|e| e.ok())
            .count();

        Ok(count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs;

    #[test]
    fn test_scan_single_file() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        fs::write(&file_path, "test content").unwrap();

        let scanner = Scanner::new();
        let files = scanner.scan(&file_path).unwrap();

        assert_eq!(files.len(), 1);
        assert!(files[0].is_file());
        assert_eq!(files[0].size, 12);
    }

    #[test]
    fn test_scan_directory_non_recursive() {
        let temp_dir = TempDir::new().unwrap();
        let dir_path = temp_dir.path();


        fs::write(dir_path.join("file1.txt"), "content1").unwrap();
        fs::write(dir_path.join("file2.txt"), "content2").unwrap();
        fs::create_dir(dir_path.join("subdir")).unwrap();

        let scanner = Scanner::new().recursive(false);
        let files = scanner.scan(dir_path).unwrap();


        assert_eq!(files.len(), 3);
    }

    #[test]
    fn test_scan_directory_recursive() {
        let temp_dir = TempDir::new().unwrap();
        let dir_path = temp_dir.path();


        fs::write(dir_path.join("file1.txt"), "content1").unwrap();
        fs::create_dir(dir_path.join("subdir")).unwrap();
        fs::write(dir_path.join("subdir").join("file2.txt"), "content2").unwrap();

        let scanner = Scanner::new().recursive(true);
        let files = scanner.scan(dir_path).unwrap();


        assert!(files.len() >= 3);
    }

    #[test]
    fn test_count_files() {
        let temp_dir = TempDir::new().unwrap();
        let dir_path = temp_dir.path();

        fs::write(dir_path.join("file1.txt"), "content1").unwrap();
        fs::write(dir_path.join("file2.txt"), "content2").unwrap();

        let scanner = Scanner::new();
        let count = scanner.count_files(dir_path).unwrap();

        assert!(count >= 2);
    }
}
