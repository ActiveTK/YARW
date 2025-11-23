use std::path::Path;


#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChangeType {

    Receive,

    #[allow(dead_code)]
    Send,

    LocalChange,

    #[allow(dead_code)]
    NoUpdate,

    Message,
}


#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileType {
    File,
    Directory,
    #[allow(dead_code)]
    Symlink,
    #[allow(dead_code)]
    Device,
    #[allow(dead_code)]
    Special,
}


#[derive(Debug, Clone)]
pub struct ItemizeChange {

    pub update_type: ChangeType,

    pub file_type: FileType,

    pub checksum_diff: bool,

    pub size_diff: bool,

    pub time_diff: bool,

    pub path: String,
}

impl ItemizeChange {

    pub fn new_file(path: &Path) -> Self {
        Self {
            update_type: ChangeType::Receive,
            file_type: FileType::File,
            checksum_diff: false,
            size_diff: true,
            time_diff: true,
            path: path.to_string_lossy().to_string(),
        }
    }


    pub fn update_file(path: &Path, size_diff: bool, time_diff: bool) -> Self {
        Self {
            update_type: ChangeType::Receive,
            file_type: FileType::File,
            checksum_diff: size_diff || time_diff,
            size_diff,
            time_diff,
            path: path.to_string_lossy().to_string(),
        }
    }


    pub fn new_directory(path: &Path) -> Self {
        Self {
            update_type: ChangeType::LocalChange,
            file_type: FileType::Directory,
            checksum_diff: false,
            size_diff: false,
            time_diff: false,
            path: path.to_string_lossy().to_string(),
        }
    }


    pub fn delete_file(path: &Path) -> Self {
        Self {
            update_type: ChangeType::Message,
            file_type: FileType::File,
            checksum_diff: false,
            size_diff: false,
            time_diff: false,
            path: path.to_string_lossy().to_string(),
        }
    }



    pub fn format(&self) -> String {
        let update_char = match self.update_type {
            ChangeType::Receive => '>',
            ChangeType::Send => '<',
            ChangeType::LocalChange => 'c',
            ChangeType::NoUpdate => '.',
            ChangeType::Message => '*',
        };

        let file_type_char = match self.file_type {
            FileType::File => 'f',
            FileType::Directory => 'd',
            FileType::Symlink => 'L',
            FileType::Device => 'D',
            FileType::Special => 'S',
        };

        let checksum_char = if self.checksum_diff { 'c' } else { '.' };
        let size_char = if self.size_diff { 's' } else { '.' };
        let time_char = if self.time_diff { 't' } else { '.' };


        let perms_char = '.';
        let owner_char = '.';
        let group_char = '.';

        format!(
            "{}{}{}{}{}{}{}{} {}",
            update_char,
            file_type_char,
            checksum_char,
            size_char,
            time_char,
            perms_char,
            owner_char,
            group_char,
            self.path
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_new_file_format() {
        let change = ItemizeChange::new_file(&PathBuf::from("test/file.txt"));
        let formatted = change.format();

        assert!(formatted.starts_with(">f"));
        assert!(formatted.contains("test/file.txt"));
    }

    #[test]
    fn test_new_directory_format() {
        let change = ItemizeChange::new_directory(&PathBuf::from("test/dir"));
        let formatted = change.format();

        assert!(formatted.starts_with("cd"));
        assert!(formatted.contains("test/dir"));
    }

    #[test]
    fn test_delete_format() {
        let change = ItemizeChange::delete_file(&PathBuf::from("test/old.txt"));
        let formatted = change.format();

        assert!(formatted.starts_with("*f"));
        assert!(formatted.contains("test/old.txt"));
    }
}
