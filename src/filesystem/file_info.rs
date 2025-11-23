use std::path::PathBuf;
use std::time::SystemTime;


#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileType {
    File,
    Directory,
    Symlink,
}


#[derive(Debug, Clone)]
pub struct FileInfo {

    pub path: PathBuf,


    pub size: u64,


    pub mtime: SystemTime,


    pub file_type: FileType,


    pub is_symlink: bool,


    pub symlink_target: Option<PathBuf>,





}

impl FileInfo {

    pub fn from_metadata(path: PathBuf, metadata: &std::fs::Metadata) -> Self {
        let file_type = if metadata.is_dir() {
            FileType::Directory
        } else if metadata.is_symlink() {
            FileType::Symlink
        } else {
            FileType::File
        };

        let is_symlink = metadata.is_symlink();
        let symlink_target = if is_symlink {
            std::fs::read_link(&path).ok()
        } else {
            None
        };

        Self {
            path,
            size: metadata.len(),
            mtime: metadata.modified().unwrap_or(SystemTime::UNIX_EPOCH),
            file_type,
            is_symlink,
            symlink_target,
        }
    }


    pub fn is_directory(&self) -> bool {
        self.file_type == FileType::Directory
    }


    #[allow(dead_code)]
    pub fn is_file(&self) -> bool {
        self.file_type == FileType::File
    }


    pub fn relative_path(&self, base: &std::path::Path) -> Option<PathBuf> {
        self.path.strip_prefix(base).ok().map(|p| p.to_path_buf())
    }


    #[allow(dead_code)]
    pub fn human_readable_size(&self) -> String {
        human_readable_size(self.size)
    }
}


pub fn human_readable_size(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];

    if bytes == 0 {
        return "0 B".to_string();
    }

    let mut size = bytes as f64;
    let mut unit_index = 0;

    while size >= 1024.0 && unit_index < UNITS.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }

    if unit_index == 0 {
        format!("{} {}", bytes, UNITS[unit_index])
    } else {
        format!("{:.2} {}", size, UNITS[unit_index])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_human_readable_size() {
        assert_eq!(human_readable_size(0), "0 B");
        assert_eq!(human_readable_size(500), "500 B");
        assert_eq!(human_readable_size(1024), "1.00 KB");
        assert_eq!(human_readable_size(1536), "1.50 KB");
        assert_eq!(human_readable_size(1048576), "1.00 MB");
        assert_eq!(human_readable_size(1073741824), "1.00 GB");
    }

    #[test]
    fn test_file_info_is_methods() {
        let file_info = FileInfo {
            path: PathBuf::from("test.txt"),
            size: 100,
            mtime: SystemTime::now(),
            file_type: FileType::File,
            is_symlink: false,
            symlink_target: None,
        };

        assert!(file_info.is_file());
        assert!(!file_info.is_directory());

        let dir_info = FileInfo {
            path: PathBuf::from("testdir"),
            size: 0,
            mtime: SystemTime::now(),
            file_type: FileType::Directory,
            is_symlink: false,
            symlink_target: None,
        };

        assert!(dir_info.is_directory());
        assert!(!dir_info.is_file());
    }
}
