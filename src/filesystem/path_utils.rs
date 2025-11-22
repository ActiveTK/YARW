use std::path::{Path, PathBuf};
use crate::error::{Result, RsyncError};

/// Windowsパスを正規化
pub fn normalize_path(path: &Path) -> Result<PathBuf> {
    // dunce crateを使ってUNCパスを正規化
    let normalized = dunce::canonicalize(path)
        .or_else(|_| dunce::simplified(path).to_path_buf().canonicalize())
        .unwrap_or_else(|_| dunce::simplified(path).to_path_buf());

    Ok(normalized)
}

/// UNCパスかどうかを判定
/// UNCパス: \\server\share または \\?\UNC\server\share
pub fn is_unc_path(path: &Path) -> bool {
    let path_str = path.to_string_lossy();
    path_str.starts_with(r"\\") && !path_str.starts_with(r"\\?\")
}

/// 長いパス形式に変換（\\?\プレフィックス）
/// Windows の MAX_PATH (260文字) 制限を回避するため
pub fn to_long_path(path: &Path) -> Result<PathBuf> {
    let path_str = path.to_string_lossy();

    // 既に長いパス形式の場合はそのまま返す
    if path_str.starts_with(r"\\?\") {
        return Ok(path.to_path_buf());
    }

    // UNCパスの場合
    if is_unc_path(path) {
        // \\server\share -> \\?\UNC\server\share
        let without_prefix = path_str.trim_start_matches(r"\\");
        return Ok(PathBuf::from(format!(r"\\?\UNC\{}", without_prefix)));
    }

    // 通常のパスの場合
    // 絶対パスに変換してから \\?\ プレフィックスを付ける
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(|e| RsyncError::Io(e))?
            .join(path)
    };

    Ok(PathBuf::from(format!(r"\\?\{}", absolute.display())))
}

/// パス区切り文字を正規化（Unix形式 → Windows形式）
/// リモート転送時などに使用
#[allow(dead_code)]
pub fn normalize_separators(path: &str) -> String {
    path.replace('/', r"\")
}

/// パス区切り文字をUnix形式に変換（Windows形式 → Unix形式）
/// リモート転送時に送信する前に使用
pub fn to_unix_separators(path: &str) -> String {
    path.replace('\\', "/")
}

/// パスが260文字制限を超えているかチェック
pub fn exceeds_max_path(path: &Path) -> bool {
    const MAX_PATH: usize = 260;
    path.to_string_lossy().len() > MAX_PATH
}

/// 大文字小文字を区別しないパス比較
#[allow(dead_code)]
pub fn paths_equal_ignore_case(path1: &Path, path2: &Path) -> bool {
    let p1 = path1.to_string_lossy().to_lowercase();
    let p2 = path2.to_string_lossy().to_lowercase();
    p1 == p2
}

/// リモートパスかどうかを判定
pub fn is_remote_path(path_str: &str) -> bool {
    // rsync:// プロトコル
    if path_str.starts_with("rsync://") {
        return true;
    }
    // Windowsの絶対パス（C:\...）やUNCパス（\\server\share）でない、かつコロンを含む場合はリモートと見なす
    let path = Path::new(path_str);
    !path.is_absolute() && !path.starts_with("\\\\") && path_str.contains(':')
}

/// デーモンパス（rsync://）かどうかを判定
pub fn is_daemon_path(path_str: &str) -> bool {
    path_str.starts_with("rsync://")
}

/// リモートパスをパースする
/// user@host:path -> (Some(("user", "host")), "path")
/// host:path     -> (Some(("", "host")), "path")
/// /local/path   -> (None, "/local/path")
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
    fn test_normalize_separators() {
        assert_eq!(normalize_separators("folder/subfolder/file.txt"), r"folder\subfolder\file.txt");
        assert_eq!(normalize_separators(r"folder\subfolder\file.txt"), r"folder\subfolder\file.txt");
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

        // 260文字を超えるパスを生成
        let long_name = "a".repeat(300);
        let long_path = Path::new(&long_name);
        assert!(exceeds_max_path(long_path));
    }

    #[test]
    fn test_paths_equal_ignore_case() {
        assert!(paths_equal_ignore_case(
            Path::new(r"C:\Folder\File.txt"),
            Path::new(r"c:\folder\file.txt")
        ));
        assert!(!paths_equal_ignore_case(
            Path::new(r"C:\Folder\File1.txt"),
            Path::new(r"C:\Folder\File2.txt")
        ));
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
