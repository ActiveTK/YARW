use std::path::{Path, PathBuf};
use std::fs::File;
use std::io::{BufRead, BufReader};
use crate::error::{Result, RsyncError};

/// ファイルリストをファイルから読み込む
///
/// `--files-from` オプションで指定されたファイルから、
/// 転送対象のファイルリストを読み込みます。
///
/// # 引数
/// * `file_path` - ファイルリストが記載されたファイルのパス
///
/// # 戻り値
/// 転送対象のファイルパスのリスト
///
/// # エラー
/// ファイルが存在しない場合や読み込みエラーの場合はエラーを返します
pub fn read_files_from(file_path: &Path) -> Result<Vec<PathBuf>> {
    let file = File::open(file_path).map_err(|e| {
        RsyncError::Io(std::io::Error::new(
            e.kind(),
            format!("Failed to open files-from file '{}': {}", file_path.display(), e)
        ))
    })?;

    let reader = BufReader::new(file);
    let mut files = Vec::new();

    for (line_num, line) in reader.lines().enumerate() {
        let line = line?;

        // 空行とコメント行（#で始まる）をスキップ
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        // パスを正規化
        let path = PathBuf::from(trimmed);

        // ファイルが存在するか確認（警告のみ、エラーにはしない）
        if !path.exists() {
            eprintln!("Warning: File listed in files-from does not exist (line {}): {}",
                line_num + 1, path.display());
        }

        files.push(path);
    }

    Ok(files)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_read_files_from() -> Result<()> {
        let mut temp_file = NamedTempFile::new()?;

        // テスト用のファイルリストを作成
        writeln!(temp_file, "file1.txt")?;
        writeln!(temp_file, "file2.txt")?;
        writeln!(temp_file, "")?;  // 空行
        writeln!(temp_file, "# コメント")?;
        writeln!(temp_file, "file3.txt")?;

        let files = read_files_from(temp_file.path())?;

        assert_eq!(files.len(), 3);
        assert_eq!(files[0], PathBuf::from("file1.txt"));
        assert_eq!(files[1], PathBuf::from("file2.txt"));
        assert_eq!(files[2], PathBuf::from("file3.txt"));

        Ok(())
    }

    #[test]
    fn test_read_files_from_nonexistent() {
        let result = read_files_from(Path::new("nonexistent_file.txt"));
        assert!(result.is_err());
    }
}
