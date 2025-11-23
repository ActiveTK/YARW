use crate::filesystem::{FileInfo, FileType};
use crate::protocol::stream::ProtocolStream;
use crate::error::Result;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::time::UNIX_EPOCH;


pub struct FileList;

impl FileList {





    pub fn encode<S: Read + Write>(stream: &mut ProtocolStream<S>, files: &[FileInfo]) -> Result<()> {

        stream.write_varint(files.len() as i64)?;


        for file in files {

            let path_str = file.path.to_string_lossy();
            stream.write_string(&path_str)?;


            stream.write_varint(file.size as i64)?;


            let mtime_secs = file.mtime.duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            stream.write_varint(mtime_secs as i64)?;


            let file_type_code = match file.file_type {
                FileType::File => 0i8,
                FileType::Directory => 1i8,
                FileType::Symlink => 2i8,
            };
            stream.write_i8(file_type_code)?;


            if file.is_symlink {
                if let Some(ref target) = file.symlink_target {
                    stream.write_string(&target.to_string_lossy())?;
                } else {
                    stream.write_string("")?;
                }
            }
        }

        stream.flush()?;
        Ok(())
    }








    pub fn decode<S: Read + Write>(stream: &mut ProtocolStream<S>) -> Result<Vec<FileInfo>> {

        let num_files = stream.read_varint()? as usize;
        let mut files = Vec::with_capacity(num_files);


        for _ in 0..num_files {

            let path_str = stream.read_string(4096)?;
            let path = PathBuf::from(path_str);


            let size = stream.read_varint()? as u64;


            let mtime_secs = stream.read_varint()? as u64;
            let mtime = UNIX_EPOCH + std::time::Duration::from_secs(mtime_secs);


            let file_type_code = stream.read_i8()?;
            let file_type = match file_type_code {
                0 => FileType::File,
                1 => FileType::Directory,
                2 => FileType::Symlink,
                _ => FileType::File,
            };


            let is_symlink = file_type == FileType::Symlink;
            let symlink_target = if is_symlink {
                let target_str = stream.read_string(4096)?;
                if target_str.is_empty() {
                    None
                } else {
                    Some(PathBuf::from(target_str))
                }
            } else {
                None
            };

            files.push(FileInfo {
                path,
                size,
                mtime,
                file_type,
                is_symlink,
                symlink_target,
            });
        }

        Ok(files)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::filesystem::{FileInfo, FileType};
    use std::io::Cursor;
    use std::path::PathBuf;
    use std::time::SystemTime;

    #[test]
    fn test_encode_decode() -> Result<()> {
        let mtime = UNIX_EPOCH + std::time::Duration::from_secs(1000000);
        let files = vec![
            FileInfo {
                path: PathBuf::from("file1.txt"),
                size: 100,
                mtime,
                file_type: FileType::File,
                is_symlink: false,
                symlink_target: None,
            },
            FileInfo {
                path: PathBuf::from("dir1"),
                size: 0,
                mtime,
                file_type: FileType::Directory,
                is_symlink: false,
                symlink_target: None,
            },
        ];

        let mut buffer = Cursor::new(Vec::new());
        let mut stream = ProtocolStream::new(&mut buffer, 31);


        FileList::encode(&mut stream, &files)?;


        stream.get_mut().set_position(0);
        let decoded_files = FileList::decode(&mut stream)?;


        assert_eq!(decoded_files.len(), files.len());


        for (original, decoded) in files.iter().zip(decoded_files.iter()) {
            assert_eq!(original.path, decoded.path);
            assert_eq!(original.size, decoded.size);
            assert_eq!(original.file_type, decoded.file_type);
            assert_eq!(original.is_symlink, decoded.is_symlink);


            let original_secs = original.mtime.duration_since(UNIX_EPOCH).unwrap().as_secs();
            let decoded_secs = decoded.mtime.duration_since(UNIX_EPOCH).unwrap().as_secs();
            assert_eq!(original_secs, decoded_secs);
        }

        Ok(())
    }

    #[test]
    fn test_encode_decode_with_symlink() -> Result<()> {
        let mtime = UNIX_EPOCH + std::time::Duration::from_secs(2000000);
        let files = vec![
            FileInfo {
                path: PathBuf::from("link1"),
                size: 0,
                mtime,
                file_type: FileType::Symlink,
                is_symlink: true,
                symlink_target: Some(PathBuf::from("/target/path")),
            },
        ];

        let mut buffer = Cursor::new(Vec::new());
        let mut stream = ProtocolStream::new(&mut buffer, 31);


        FileList::encode(&mut stream, &files)?;


        stream.get_mut().set_position(0);
        let decoded_files = FileList::decode(&mut stream)?;

        assert_eq!(decoded_files.len(), 1);
        assert_eq!(decoded_files[0].path, files[0].path);
        assert_eq!(decoded_files[0].is_symlink, true);
        assert_eq!(decoded_files[0].symlink_target, files[0].symlink_target);

        Ok(())
    }
}
