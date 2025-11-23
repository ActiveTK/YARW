use std::path::{Path, PathBuf};
use std::fs::File;
use std::io::{BufRead, BufReader};
use crate::error::{Result, RsyncError};
use crate::output::VerboseOutput;














pub fn read_files_from(file_path: &Path) -> Result<Vec<PathBuf>> {
    let file = File::open(file_path).map_err(|e| {
        RsyncError::Io(std::io::Error::new(
            e.kind(),
            format!("Failed to open files-from file '{}': {}", file_path.display(), e)
        ))
    })?;

    let reader = BufReader::new(file);
    let mut files = Vec::new();
    let verbose = VerboseOutput::new(1, false);

    for (line_num, line) in reader.lines().enumerate() {
        let line = line?;


        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }


        let path = PathBuf::from(trimmed);


        if !path.exists() {
            verbose.print_warning(&format!("File listed in files-from does not exist (line {}): {}",
                line_num + 1, path.display()));
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


        writeln!(temp_file, "file1.txt")?;
        writeln!(temp_file, "file2.txt")?;
        writeln!(temp_file, "")?;
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
