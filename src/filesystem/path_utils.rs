use std::path::{Path, PathBuf};
use crate::error::{Result, RsyncError};


pub fn normalize_path(path: &Path) -> Result<PathBuf> {

    let normalized = dunce::canonicalize(path)
        .or_else(|_| dunce::simplified(path).to_path_buf().canonicalize())
        .unwrap_or_else(|_| dunce::simplified(path).to_path_buf());

    Ok(normalized)
}



pub fn is_unc_path(path: &Path) -> bool {
    let path_str = path.to_string_lossy();
    path_str.starts_with(r"\\") && !path_str.starts_with(r"\\?\")
}



pub fn to_long_path(path: &Path) -> Result<PathBuf> {
    let path_str = path.to_string_lossy();


    if path_str.starts_with(r"\\?\") {
        return Ok(path.to_path_buf());
    }


    if is_unc_path(path) {

        let without_prefix = path_str.trim_start_matches(r"\\");
        return Ok(PathBuf::from(format!(r"\\?\UNC\{}", without_prefix)));
    }



    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(|e| RsyncError::Io(e))?
            .join(path)
    };

    Ok(PathBuf::from(format!(r"\\?\{}", absolute.display())))
}



pub fn to_unix_separators(path: &str) -> String {
    path.replace('\\', "/")
}


pub fn exceeds_max_path(path: &Path) -> bool {
    const MAX_PATH: usize = 260;
    path.to_string_lossy().len() > MAX_PATH
}


pub fn is_remote_path(path_str: &str) -> bool {

    if path_str.starts_with("rsync://") {
        return true;
    }

    let path = Path::new(path_str);
    !path.is_absolute() && !path.starts_with("\\\\") && path_str.contains(':')
}


pub fn is_daemon_path(path_str: &str) -> bool {
    path_str.starts_with("rsync://")
}





pub fn parse_remote_path(path: &str) -> (Option<(String, String)>, String) {
    if !is_remote_path(path) {
        return (None, path.to_string());
    }

    let (host_part, path_part) = path.split_once(':').unwrap_or((path, ""));

    let user_host = if let Some((user, host)) = host_part.split_once('@') {
        (user.to_string(), host.to_string())
    } else {
        ("".to_string(), host_part.to_string())
    };

    (Some(user_host), path_part.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_unc_path() {
        assert!(is_unc_path(Path::new(r"\\server\share")));
        assert!(is_unc_path(Path::new(r"\\server\share\folder")));
        assert!(!is_unc_path(Path::new(r"C:\folder")));
        assert!(!is_unc_path(Path::new(r"\\?\C:\folder")));
    }

    #[test]
    fn test_to_unix_separators() {
        assert_eq!(to_unix_separators(r"folder\subfolder\file.txt"), "folder/subfolder/file.txt");
        assert_eq!(to_unix_separators("folder/subfolder/file.txt"), "folder/subfolder/file.txt");
    }

    #[test]
    fn test_exceeds_max_path() {
        let short_path = Path::new("short.txt");
        assert!(!exceeds_max_path(short_path));


        let long_name = "a".repeat(300);
        let long_path = Path::new(&long_name);
        assert!(exceeds_max_path(long_path));
    }

    #[test]
    fn test_is_remote_path() {
        assert!(is_remote_path("user@host:/path/to/file"));
        assert!(is_remote_path("host:/path/to/file"));
        assert!(!is_remote_path("C:\\Users\\user\\file.txt"));
        assert!(!is_remote_path("/path/to/file"));
        assert!(!is_remote_path("\\\\server\\share"));
    }

    #[test]
    fn test_parse_remote_path() {
        let (user_host, path) = parse_remote_path("user@host:/path/to/file");
        assert_eq!(user_host, Some(("user".to_string(), "host".to_string())));
        assert_eq!(path, "/path/to/file");

        let (user_host, path) = parse_remote_path("host:/path/to/file");
        assert_eq!(user_host, Some(("".to_string(), "host".to_string())));
        assert_eq!(path, "/path/to/file");

        let (user_host, path) = parse_remote_path("C:\\Users\\user\\file.txt");
        assert_eq!(user_host, None);
        assert_eq!(path, "C:\\Users\\user\\file.txt");
    }
}
