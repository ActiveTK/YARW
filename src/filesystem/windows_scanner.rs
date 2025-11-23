




#[cfg(windows)]
use windows::Win32::Foundation::{HANDLE, INVALID_HANDLE_VALUE};
#[cfg(windows)]
use windows::Win32::Storage::FileSystem::{
    FindFirstFileExW, FindNextFileW, FindClose,
    FILE_ATTRIBUTE_DIRECTORY, FILE_ATTRIBUTE_REPARSE_POINT,
    WIN32_FIND_DATAW,
    FindExInfoBasic, FindExSearchNameMatch,
    FIND_FIRST_EX_LARGE_FETCH,
};
use std::path::Path;
use std::time::SystemTime;
use crate::error::{Result, RsyncError};
use crate::filesystem::FileInfo;




#[cfg(windows)]
pub struct WindowsScanner {
    recursive: bool,
    follow_symlinks: bool,
}

#[cfg(windows)]
impl WindowsScanner {

    pub fn new() -> Self {
        Self {
            recursive: false,
            follow_symlinks: false,
        }
    }


    pub fn recursive(mut self, recursive: bool) -> Self {
        self.recursive = recursive;
        self
    }


    pub fn follow_symlinks(mut self, follow: bool) -> Self {
        self.follow_symlinks = follow;
        self
    }


    pub fn scan(&self, path: &Path) -> Result<Vec<FileInfo>> {
        let mut results = Vec::new();
        self.scan_internal(path, path, &mut results)?;
        Ok(results)
    }


    fn scan_internal(
        &self,
        base_path: &Path,
        current_path: &Path,
        results: &mut Vec<FileInfo>,
    ) -> Result<()> {

        let search_pattern = current_path.join("*");
        let search_pattern_wide = to_wide_string(search_pattern.to_str().unwrap());

        let mut find_data: WIN32_FIND_DATAW = unsafe { std::mem::zeroed() };



        let handle = unsafe {
            FindFirstFileExW(
                windows::core::PCWSTR(search_pattern_wide.as_ptr()),
                FindExInfoBasic,
                &mut find_data as *mut _ as *mut _,
                FindExSearchNameMatch,
                None,
                FIND_FIRST_EX_LARGE_FETCH,
            )
        }.map_err(|_| RsyncError::Io(std::io::Error::last_os_error()))?;

        if handle == INVALID_HANDLE_VALUE {
            return Err(RsyncError::Io(std::io::Error::last_os_error()));
        }


        let _guard = HandleGuard(handle);

        loop {
            let file_name = from_wide_string(&find_data.cFileName);


            if file_name != "." && file_name != ".." {
                let full_path = current_path.join(&file_name);
                let is_directory = (find_data.dwFileAttributes & FILE_ATTRIBUTE_DIRECTORY.0) != 0;
                let is_symlink = (find_data.dwFileAttributes & FILE_ATTRIBUTE_REPARSE_POINT.0) != 0;


                let file_info = FileInfo {
                    path: full_path.clone(),
                    size: if is_directory {
                        0
                    } else {
                        ((find_data.nFileSizeHigh as u64) << 32) | (find_data.nFileSizeLow as u64)
                    },
                    mtime: filetime_to_systemtime(&find_data.ftLastWriteTime),
                    file_type: if is_directory {
                        crate::filesystem::FileType::Directory
                    } else if is_symlink {
                        crate::filesystem::FileType::Symlink
                    } else {
                        crate::filesystem::FileType::File
                    },
                    is_symlink,
                    symlink_target: None,
                };

                results.push(file_info);


                if is_directory && self.recursive && (!is_symlink || self.follow_symlinks) {
                    self.scan_internal(base_path, &full_path, results)?;
                }
            }


            let result = unsafe { FindNextFileW(handle, &mut find_data) };
            if result.is_err() {

                let last_error = std::io::Error::last_os_error();
                if last_error.raw_os_error() == Some(18) {
                    break;
                } else {
                    return Err(RsyncError::Io(last_error));
                }
            }
        }

        Ok(())
    }
}

#[cfg(windows)]
impl Default for WindowsScanner {
    fn default() -> Self {
        Self::new()
    }
}


#[cfg(windows)]
struct HandleGuard(HANDLE);

#[cfg(windows)]
impl Drop for HandleGuard {
    fn drop(&mut self) {
        unsafe {
            let _ = FindClose(self.0);
        }
    }
}


#[cfg(windows)]
fn to_wide_string(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}


#[cfg(windows)]
fn from_wide_string(wide: &[u16]) -> String {
    let len = wide.iter().position(|&c| c == 0).unwrap_or(wide.len());
    String::from_utf16_lossy(&wide[..len])
}


#[cfg(windows)]
fn filetime_to_systemtime(ft: &windows::Win32::Foundation::FILETIME) -> SystemTime {



    const TICKS_PER_SECOND: u64 = 10_000_000;
    const EPOCH_DIFF_SECONDS: u64 = 11_644_473_600;

    let ticks = ((ft.dwHighDateTime as u64) << 32) | (ft.dwLowDateTime as u64);
    let seconds = ticks / TICKS_PER_SECOND;

    if seconds > EPOCH_DIFF_SECONDS {
        let unix_seconds = seconds - EPOCH_DIFF_SECONDS;
        let nanos = ((ticks % TICKS_PER_SECOND) * 100) as u32;

        SystemTime::UNIX_EPOCH + std::time::Duration::new(unix_seconds, nanos)
    } else {
        SystemTime::UNIX_EPOCH
    }
}


#[cfg(not(windows))]
pub struct WindowsScanner;

#[cfg(not(windows))]
impl WindowsScanner {
    pub fn new() -> Self {
        Self
    }

    pub fn recursive(self, _recursive: bool) -> Self {
        self
    }

    pub fn follow_symlinks(self, _follow: bool) -> Self {
        self
    }

    pub fn scan(&self, _path: &Path) -> Result<Vec<FileInfo>> {
        Err(RsyncError::Io(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "WindowsScanner is only available on Windows",
        )))
    }
}

#[cfg(not(windows))]
impl Default for WindowsScanner {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
#[cfg(windows)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs;

    #[test]
    fn test_windows_scanner_basic() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let test_file = temp_dir.path().join("test.txt");
        fs::write(&test_file, "test content")?;

        let scanner = WindowsScanner::new();
        let results = scanner.scan(temp_dir.path())?;

        assert_eq!(results.len(), 1);
        assert!(results[0].path.ends_with("test.txt"));
        assert_eq!(results[0].size, 12);

        Ok(())
    }

    #[test]
    fn test_windows_scanner_recursive() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let sub_dir = temp_dir.path().join("subdir");
        fs::create_dir(&sub_dir)?;
        fs::write(sub_dir.join("nested.txt"), "nested")?;

        let scanner = WindowsScanner::new().recursive(true);
        let results = scanner.scan(temp_dir.path())?;


        assert!(results.len() >= 2);

        Ok(())
    }
}
