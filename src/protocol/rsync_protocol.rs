use std::io::{Read, Write};
use byteorder::{ReadBytesExt, WriteBytesExt, LittleEndian};
use crate::error::{Result, RsyncError};

pub const CF_INC_RECURSE: u8 = 1 << 0;
pub const CF_SYMLINK_TIMES: u8 = 1 << 1;
pub const CF_SYMLINK_ICONV: u8 = 1 << 2;
pub const CF_SAFE_FLIST: u8 = 1 << 3;
pub const CF_AVOID_XATTR_OPTIM: u8 = 1 << 4;
pub const CF_CHKSUM_SEED_FIX: u8 = 1 << 5;
pub const CF_INPLACE_PARTIAL_DIR: u8 = 1 << 6;
pub const CF_VARINT_FLIST_FLAGS: u8 = 1 << 7;

pub const XMIT_TOP_DIR: u16 = 1 << 0;
pub const XMIT_SAME_MODE: u16 = 1 << 1;
pub const XMIT_EXTENDED_FLAGS: u16 = 1 << 2;
pub const XMIT_SAME_UID: u16 = 1 << 3;
pub const XMIT_SAME_GID: u16 = 1 << 4;
pub const XMIT_SAME_NAME: u16 = 1 << 5;
pub const XMIT_LONG_NAME: u16 = 1 << 6;
pub const XMIT_SAME_TIME: u16 = 1 << 7;
pub const XMIT_SAME_RDEV_MAJOR: u16 = 1 << 8;
pub const XMIT_HLINKED: u16 = 1 << 9;
pub const XMIT_HLINK_FIRST: u16 = 1 << 10;
pub const XMIT_IO_ERROR_ENDLIST: u16 = 1 << 11;
pub const XMIT_SAME_DEV_MAJOR: u16 = 1 << 12;
pub const XMIT_RDEV_MINOR_8_PRE30: u16 = 1 << 13;
pub const XMIT_GROUP_NAME_FOLLOWS: u16 = 1 << 14;
pub const XMIT_USER_NAME_FOLLOWS: u16 = 1 << 15;

pub const XMIT_SAME_UID_8: u16 = 1 << 0;
pub const XMIT_SAME_GID_8: u16 = 1 << 1;
pub const XMIT_MOD_NSEC: u16 = 1 << 2;
pub const XMIT_SAME_ATIME: u16 = 1 << 3;
pub const XMIT_UNUSED_4: u16 = 1 << 4;
pub const XMIT_SAME_ACL: u16 = 1 << 5;
pub const XMIT_SAME_XATTR: u16 = 1 << 6;
pub const XMIT_CRTIME_EQ_MTIME: u16 = 1 << 7;

pub const MIN_FILECNT_LOOKAHEAD: usize = 1000;

pub struct CompatFlags {
    pub flags: u8,
}

impl CompatFlags {
    pub fn new_for_protocol_31() -> Self {
        let mut flags = 0u8;
        flags |= CF_SAFE_FLIST;
        flags |= CF_CHKSUM_SEED_FIX;
        flags |= CF_VARINT_FLIST_FLAGS;
        Self { flags }
    }

    pub fn write<W: Write>(&self, writer: &mut W) -> Result<()> {
        writer.write_u8(self.flags)?;
        Ok(())
    }

    pub fn read<R: Read>(reader: &mut R) -> Result<Self> {
        let flags = reader.read_u8()?;
        Ok(Self { flags })
    }

    pub fn has_flag(&self, flag: u8) -> bool {
        (self.flags & flag) != 0
    }
}

pub fn write_varint<W: Write>(writer: &mut W, val: i64) -> Result<()> {
    match val {
        0..=127 => {
            writer.write_u8(val as u8)?;
            Ok(())
        }
        -128..=-1 => {
            writer.write_u8(0x80)?;
            writer.write_i8(val as i8)?;
            Ok(())
        }
        128..=32767 => {
            writer.write_u8(0x81)?;
            writer.write_i16::<LittleEndian>(val as i16)?;
            Ok(())
        }
        -32768..=-129 => {
            writer.write_u8(0x82)?;
            writer.write_i16::<LittleEndian>(val as i16)?;
            Ok(())
        }
        32768..=2147483647 | -2147483648..=-32769 => {
            writer.write_u8(0x83)?;
            writer.write_i32::<LittleEndian>(val as i32)?;
            Ok(())
        }
        _ => {
            writer.write_u8(0x84)?;
            writer.write_i64::<LittleEndian>(val)?;
            Ok(())
        }
    }
}

pub fn read_varint<R: Read>(reader: &mut R) -> Result<i64> {
    const INT_BYTE_EXTRA: [usize; 64] = [
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
        2, 2, 2, 2, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 5, 6,
    ];

    let ch = reader.read_u8()?;
    let extra = INT_BYTE_EXTRA[(ch / 4) as usize];

    if extra == 0 {
        return Ok(ch as i64);
    }

    let bit = 1u8 << (8 - extra);
    let mut bytes = vec![0u8; extra + 1];

    reader.read_exact(&mut bytes[0..extra])?;
    bytes[extra] = ch & (bit - 1);

    let mut result = i32::from_le_bytes([
        bytes.get(0).copied().unwrap_or(0),
        bytes.get(1).copied().unwrap_or(0),
        bytes.get(2).copied().unwrap_or(0),
        bytes.get(3).copied().unwrap_or(0),
    ]);

    if result & 0x80000000_u32 as i32 != 0 {
        result |= !0x7fffffff;
    }

    Ok(result as i64)
}

pub fn write_varlong30<W: Write>(writer: &mut W, val: i64) -> Result<()> {
    if val < 0 || val > 0x7FFFFFFF {
        writer.write_u8(0xFFu8)?;
        writer.write_i64::<LittleEndian>(val)?;
    } else if val < 0x1000000 {
        writer.write_u8((val & 0xFF) as u8)?;
        writer.write_u8(((val >> 8) & 0xFF) as u8)?;
        writer.write_u8(((val >> 16) & 0xFF) as u8)?;
    } else {
        writer.write_u8(0xFEu8)?;
        writer.write_i32::<LittleEndian>(val as i32)?;
    }
    Ok(())
}

pub fn read_varlong30<R: Read>(reader: &mut R) -> Result<i64> {
    let b1 = reader.read_u8()? as i64;
    let b2 = reader.read_u8()? as i64;
    let b3 = reader.read_u8()? as i64;

    if b1 == 0xFF {
        let high = reader.read_i32::<LittleEndian>()? as i64;
        let low = (b2 | (b3 << 8)) as i64;
        return Ok((high << 16) | low);
    }

    if b1 == 0xFE {
        let val = reader.read_i8()? as i64;
        return Ok(((val as i64) << 16) | (b2 | (b3 << 8)));
    }

    Ok(b1 | (b2 << 8) | (b3 << 16))
}

pub fn write_varint30<W: Write>(writer: &mut W, val: i64) -> Result<()> {
    if val < 0 || val >= 0x40000000 {
        writer.write_u8(0xFFu8)?;
        writer.write_i64::<LittleEndian>(val)?;
    } else if val < 0x10000 {
        writer.write_u8((val & 0xFF) as u8)?;
        writer.write_u8(((val >> 8) & 0xFF) as u8)?;
    } else {
        writer.write_u8(0xFEu8)?;
        writer.write_u8((val & 0xFF) as u8)?;
        writer.write_u8(((val >> 8) & 0xFF) as u8)?;
        writer.write_u8(((val >> 16) & 0xFF) as u8)?;
        writer.write_u8(((val >> 24) & 0xFF) as u8)?;
    }
    Ok(())
}

pub fn read_varint30<R: Read>(reader: &mut R) -> Result<i64> {
    let b1 = reader.read_u8()? as i64;
    eprintln!("[VARINT30] Read b1={:#04x}", b1);

    if b1 == 0xFF {
        let b2 = reader.read_u8()? as i64;
        let high = reader.read_i64::<LittleEndian>()?;
        let result = (high << 8) | b2;
        eprintln!("[VARINT30] Mode 0xFF: b2={}, high={}, result={}", b2, high, result);
        return Ok(result);
    }

    if b1 == 0xFE {
        let mut bytes = [0u8; 4];
        reader.read_exact(&mut bytes)?;
        let result = i32::from_le_bytes(bytes) as i64;
        eprintln!("[VARINT30] Mode 0xFE: bytes={:?}, result={}", bytes, result);
        return Ok(result);
    }

    let b2 = reader.read_u8()? as i64;
    eprintln!("[VARINT30] Read b2={:#04x}", b2);

    if b1 < 0x80 {
        let result = b1 | (b2 << 8);
        eprintln!("[VARINT30] Mode <0x80: result={}", result);
        Ok(result)
    } else {
        let result = ((b1 & 0x7F) << 7) | b2;
        eprintln!("[VARINT30] Mode >=0x80: result={}", result);
        Ok(result)
    }
}

pub fn write_shortint<W: Write>(writer: &mut W, val: u16) -> Result<()> {
    writer.write_u8((val & 0xFF) as u8)?;
    writer.write_u8(((val >> 8) & 0xFF) as u8)?;
    Ok(())
}

pub fn read_shortint<R: Read>(reader: &mut R) -> Result<u16> {
    let b1 = reader.read_u8()? as u16;
    let b2 = reader.read_u8()? as u16;
    Ok(b1 | (b2 << 8))
}

pub fn write_vstring<W: Write>(writer: &mut W, s: &str) -> Result<()> {
    let bytes = s.as_bytes();
    let len = bytes.len();

    if len > 0x7FFF {
        return Err(RsyncError::Other(format!("vstring too long: {}", len)));
    }

    if len > 0x7F {
        writer.write_u8((len / 0x100 + 0x80) as u8)?;
    }
    writer.write_u8((len & 0xFF) as u8)?;

    if len > 0 {
        writer.write_all(bytes)?;
    }

    Ok(())
}

pub fn read_vstring<R: Read>(reader: &mut R) -> Result<String> {
    let mut len = reader.read_u8()? as usize;

    if (len & 0x80) != 0 {
        len = (len & !0x80) * 0x100 + reader.read_u8()? as usize;
    }

    if len > 0x7FFF {
        return Err(RsyncError::Other(format!("vstring too long: {}", len)));
    }

    if len == 0 {
        return Ok(String::new());
    }

    let mut buf = vec![0u8; len];
    reader.read_exact(&mut buf)?;

    String::from_utf8(buf)
        .map_err(|e| RsyncError::Other(format!("Invalid UTF-8 in vstring: {}", e)))
}
