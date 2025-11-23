use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use byteorder::{ReadBytesExt, WriteBytesExt};
use crate::error::Result;
use crate::filesystem::FileInfo;
use super::rsync_protocol::*;

pub struct FileEntry {
    pub path: PathBuf,
    pub mode: u32,
    pub len: u64,
    pub modtime: i64,
    pub uid: u32,
    pub gid: u32,
    pub is_dir: bool,
    pub is_symlink: bool,
    pub symlink_target: Option<String>,
}

impl FileEntry {
    pub fn from_file_info(info: &FileInfo, base_path: &Path) -> Self {
        let path = if let Ok(stripped) = info.path.strip_prefix(base_path) {
            stripped.to_path_buf()
        } else {
            info.path.clone()
        };

        let modtime = info.mtime
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;

        Self {
            path,
            mode: 0o644,
            len: info.size,
            modtime,
            uid: 0,
            gid: 0,
            is_dir: info.is_directory(),
            is_symlink: info.is_symlink,
            symlink_target: info.symlink_target.as_ref().map(|p| p.to_string_lossy().to_string()),
        }
    }
}

struct FileListState {
    last_name: String,
    last_mode: u32,
    last_modtime: i64,
    last_uid: u32,
    last_gid: u32,
}

impl FileListState {
    fn new() -> Self {
        Self {
            last_name: String::new(),
            last_mode: 0,
            last_modtime: 0,
            last_uid: 0,
            last_gid: 0,
        }
    }
}

pub fn send_file_list<W: Write>(
    writer: &mut W,
    files: &[FileInfo],
    base_path: &Path,
    protocol_version: i32,
    compat_flags: &CompatFlags,
) -> Result<()> {
    let mut state = FileListState::new();
    let use_varint_flags = compat_flags.has_flag(CF_VARINT_FLIST_FLAGS);

    for file in files {
        let entry = FileEntry::from_file_info(file, base_path);
        send_file_entry(writer, &entry, &mut state, protocol_version, use_varint_flags)?;
    }

    write_end_of_flist(writer, use_varint_flags)?;
    Ok(())
}

fn send_file_entry<W: Write>(
    writer: &mut W,
    entry: &FileEntry,
    state: &mut FileListState,
    protocol_version: i32,
    use_varint_flags: bool,
) -> Result<()> {
    let mut flags: u16 = 0;
    let _xflags: u16 = 0;

    let path_str = entry.path.to_string_lossy();
    let path_bytes = path_str.as_bytes();

    let common_prefix_len = find_common_prefix(&state.last_name, &path_str);

    if common_prefix_len > 0 {
        flags |= XMIT_SAME_NAME;
    }

    let suffix_len = path_bytes.len() - common_prefix_len;
    if suffix_len > 255 {
        flags |= XMIT_LONG_NAME;
    }

    if entry.modtime == state.last_modtime {
        flags |= XMIT_SAME_TIME;
    }

    if entry.mode == state.last_mode && state.last_mode != 0 {
        flags |= XMIT_SAME_MODE;
    }

    if entry.uid == state.last_uid && state.last_uid != 0 {
        flags |= XMIT_SAME_UID;
    }

    if entry.gid == state.last_gid && state.last_gid != 0 {
        flags |= XMIT_SAME_GID;
    }

    if entry.is_dir {
        flags |= XMIT_TOP_DIR;
    }

    if protocol_version >= 28 && (flags >> 8) != 0 {
        flags |= XMIT_EXTENDED_FLAGS;
    }

    if flags == 0 {
        flags |= XMIT_TOP_DIR;
    }

    if use_varint_flags {
        write_varint(writer, flags as i64)?;
        if (flags & XMIT_EXTENDED_FLAGS) != 0 {
            write_varint(writer, _xflags as i64)?;
        }
    } else if protocol_version >= 28 {
        if (flags & XMIT_EXTENDED_FLAGS) != 0 {
            write_shortint(writer, flags)?;
        } else {
            writer.write_u8(flags as u8)?;
        }
    } else {
        writer.write_u8(flags as u8)?;
    }

    if (flags & XMIT_SAME_NAME) != 0 {
        writer.write_u8(common_prefix_len as u8)?;
    }

    if (flags & XMIT_LONG_NAME) != 0 {
        write_varint30(writer, suffix_len as i64)?;
    } else {
        writer.write_u8(suffix_len as u8)?;
    }

    writer.write_all(&path_bytes[common_prefix_len..])?;

    write_varlong30(writer, entry.len as i64)?;

    if (flags & XMIT_SAME_TIME) == 0 {
        if protocol_version >= 30 {
            write_varlong30(writer, entry.modtime)?;
        } else {
            writer.write_i32::<byteorder::LittleEndian>(entry.modtime as i32)?;
        }
    }

    if (flags & XMIT_SAME_MODE) == 0 {
        writer.write_u32::<byteorder::LittleEndian>(entry.mode)?;
    }

    if (flags & XMIT_SAME_UID) == 0 {
        if protocol_version >= 30 {
            write_varint(writer, entry.uid as i64)?;
        } else {
            writer.write_u32::<byteorder::LittleEndian>(entry.uid)?;
        }
    }

    if (flags & XMIT_SAME_GID) == 0 {
        if protocol_version >= 30 {
            write_varint(writer, entry.gid as i64)?;
        } else {
            writer.write_u32::<byteorder::LittleEndian>(entry.gid)?;
        }
    }

    if entry.is_symlink {
        if let Some(ref target) = entry.symlink_target {
            let target_bytes = target.as_bytes();
            write_varint30(writer, target_bytes.len() as i64)?;
            writer.write_all(target_bytes)?;
        }
    }

    state.last_name = path_str.to_string();
    state.last_mode = entry.mode;
    state.last_modtime = entry.modtime;
    state.last_uid = entry.uid;
    state.last_gid = entry.gid;

    Ok(())
}

pub fn recv_file_list<R: Read>(
    reader: &mut R,
    protocol_version: i32,
    compat_flags: &CompatFlags,
) -> Result<Vec<FileEntry>> {
    let mut files = Vec::new();
    let mut state = FileListState::new();
    let use_varint_flags = compat_flags.has_flag(CF_VARINT_FLIST_FLAGS);

    loop {
        match recv_file_entry(reader, &mut state, protocol_version, use_varint_flags) {
            Ok(Some(entry)) => files.push(entry),
            Ok(None) => break,
            Err(e) => return Err(e),
        }
    }

    Ok(files)
}

fn recv_file_entry<R: Read>(
    reader: &mut R,
    state: &mut FileListState,
    protocol_version: i32,
    use_varint_flags: bool,
) -> Result<Option<FileEntry>> {
    let flags = if use_varint_flags {
        let f = read_varint(reader)? as u16;
        if f == 0 {
            return Ok(None);
        }
        f
    } else if protocol_version >= 28 {
        let b1 = reader.read_u8()? as u16;
        if b1 == 0 {
            return Ok(None);
        }
        if (b1 & (XMIT_EXTENDED_FLAGS as u16)) != 0 {
            let b2 = reader.read_u8()? as u16;
            b1 | (b2 << 8)
        } else {
            b1
        }
    } else {
        let f = reader.read_u8()? as u16;
        if f == 0 {
            return Ok(None);
        }
        f
    };

    let _xflags = if use_varint_flags && (flags & XMIT_EXTENDED_FLAGS) != 0 {
        read_varint(reader)? as u16
    } else {
        0
    };

    let common_prefix_len = if (flags & XMIT_SAME_NAME) != 0 {
        reader.read_u8()? as usize
    } else {
        0
    };

    let suffix_len = if (flags & XMIT_LONG_NAME) != 0 {
        read_varint30(reader)? as usize
    } else {
        reader.read_u8()? as usize
    };

    let mut path_bytes = vec![0u8; suffix_len];
    reader.read_exact(&mut path_bytes)?;

    let mut full_name = String::new();
    if common_prefix_len > 0 {
        full_name.push_str(&state.last_name[..common_prefix_len]);
    }
    full_name.push_str(&String::from_utf8_lossy(&path_bytes));

    let len = read_varlong30(reader)? as u64;

    let modtime = if (flags & XMIT_SAME_TIME) != 0 {
        state.last_modtime
    } else if protocol_version >= 30 {
        read_varlong30(reader)?
    } else {
        reader.read_i32::<byteorder::LittleEndian>()? as i64
    };

    let mode = if (flags & XMIT_SAME_MODE) != 0 {
        state.last_mode
    } else {
        reader.read_u32::<byteorder::LittleEndian>()?
    };

    let uid = if (flags & XMIT_SAME_UID) != 0 {
        state.last_uid
    } else if protocol_version >= 30 {
        read_varint(reader)? as u32
    } else {
        reader.read_u32::<byteorder::LittleEndian>()?
    };

    let gid = if (flags & XMIT_SAME_GID) != 0 {
        state.last_gid
    } else if protocol_version >= 30 {
        read_varint(reader)? as u32
    } else {
        reader.read_u32::<byteorder::LittleEndian>()?
    };

    let is_dir = (flags & XMIT_TOP_DIR) != 0;
    let is_symlink = (mode & 0o170000) == 0o120000;

    let symlink_target = if is_symlink {
        let target_len = read_varint30(reader)? as usize;
        let mut target_bytes = vec![0u8; target_len];
        reader.read_exact(&mut target_bytes)?;
        Some(String::from_utf8_lossy(&target_bytes).to_string())
    } else {
        None
    };

    state.last_name = full_name.clone();
    state.last_mode = mode;
    state.last_modtime = modtime;
    state.last_uid = uid;
    state.last_gid = gid;

    Ok(Some(FileEntry {
        path: PathBuf::from(full_name),
        mode,
        len,
        modtime,
        uid,
        gid,
        is_dir,
        is_symlink,
        symlink_target,
    }))
}

fn write_end_of_flist<W: Write>(writer: &mut W, use_varint_flags: bool) -> Result<()> {
    if use_varint_flags {
        write_varint(writer, 0)?;
    } else {
        writer.write_u8(0)?;
    }
    Ok(())
}

fn find_common_prefix(s1: &str, s2: &str) -> usize {
    s1.bytes()
        .zip(s2.bytes())
        .take_while(|(a, b)| a == b)
        .count()
}
