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
pub const XMIT_USER_NAME_FOLLOWS: u16 = 1 << 10;
pub const XMIT_GROUP_NAME_FOLLOWS: u16 = 1 << 11;
pub const XMIT_HLINK_FIRST: u16 = 1 << 12;
pub const XMIT_MOD_NSEC: u16 = 1 << 13;
pub const XMIT_SAME_ATIME: u16 = 1 << 14;

pub const ITEM_REPORT_ATIME: u16 = 1 << 0;
pub const ITEM_REPORT_CHANGE: u16 = 1 << 1;
pub const ITEM_REPORT_SIZE: u16 = 1 << 2;
pub const ITEM_REPORT_TIMEFAIL: u16 = 1 << 2;
pub const ITEM_REPORT_TIME: u16 = 1 << 3;
pub const ITEM_REPORT_PERMS: u16 = 1 << 4;
pub const ITEM_REPORT_OWNER: u16 = 1 << 5;
pub const ITEM_REPORT_GROUP: u16 = 1 << 6;
pub const ITEM_REPORT_ACL: u16 = 1 << 7;
pub const ITEM_REPORT_XATTR: u16 = 1 << 8;
pub const ITEM_REPORT_CRTIME: u16 = 1 << 10;
pub const ITEM_BASIS_TYPE_FOLLOWS: u16 = 1 << 11;
pub const ITEM_XNAME_FOLLOWS: u16 = 1 << 12;
pub const ITEM_IS_NEW: u16 = 1 << 13;
pub const ITEM_LOCAL_CHANGE: u16 = 1 << 14;
pub const ITEM_TRANSFER: u16 = 1 << 15;

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

pub fn read_varlong<R: Read>(reader: &mut R, min_bytes: usize) -> Result<i64> {
    const INT_BYTE_EXTRA: [usize; 64] = [
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
        2, 2, 2, 2, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 5, 6,
    ];

    let mut b2 = vec![0u8; min_bytes];
    reader.read_exact(&mut b2)?;
    eprintln!("[VARLONG] min_bytes={}, b2={:02x?}", min_bytes, b2);

    let mut u_b = [0u8; 9];

    for i in 0..min_bytes-1 {
        u_b[i] = b2[i + 1];
    }

    let extra = INT_BYTE_EXTRA[(b2[0] / 4) as usize];
    eprintln!("[VARLONG] b2[0]={:#04x}, extra={}", b2[0], extra);

    if extra > 0 {
        let bit = 1u8 << (8 - extra);
        let mut extra_bytes = vec![0u8; extra];
        reader.read_exact(&mut extra_bytes)?;
        eprintln!("[VARLONG] extra_bytes={:02x?}", extra_bytes);

        for i in 0..extra {
            u_b[min_bytes - 1 + i] = extra_bytes[i];
        }

        u_b[min_bytes + extra - 1] = b2[0] & (bit - 1);
    } else {
        u_b[min_bytes + extra - 1] = b2[0];
    }

    eprintln!("[VARLONG] u_b={:02x?}", u_b);
    let result = i64::from_le_bytes([
        u_b[0], u_b[1], u_b[2], u_b[3],
        u_b[4], u_b[5], u_b[6], u_b[7],
    ]);
    eprintln!("[VARLONG] result={}", result);

    Ok(result)
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
    eprintln!("[VARLONG30] Read bytes: {:#04x} {:#04x} {:#04x}", b1, b2, b3);

    if b1 == 0xFF {
        let high = reader.read_i32::<LittleEndian>()? as i64;
        let low = (b2 | (b3 << 8)) as i64;
        let result = (high << 16) | low;
        eprintln!("[VARLONG30] Mode 0xFF: result={}", result);
        return Ok(result);
    }

    if b1 == 0xFE {
        let val = reader.read_i8()? as i64;
        let result = ((val as i64) << 16) | (b2 | (b3 << 8));
        eprintln!("[VARLONG30] Mode 0xFE: result={}", result);
        return Ok(result);
    }

    let result = b1 | (b2 << 8) | (b3 << 16);
    eprintln!("[VARLONG30] Normal: result={}", result);
    Ok(result)
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
    const INT_BYTE_EXTRA: [usize; 64] = [
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
        2, 2, 2, 2, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 5, 6,
    ];

    let ch = reader.read_u8()?;
    let extra = INT_BYTE_EXTRA[(ch / 4) as usize];

    if extra == 0 {
        eprintln!("[VARINT30] Single byte: {}", ch);
        return Ok(ch as i64);
    }

    let bit = 1u8 << (8 - extra);
    let mut bytes = vec![0u8; extra + 1];

    reader.read_exact(&mut bytes[0..extra])?;
    bytes[extra] = ch & (bit - 1);

    let result = i32::from_le_bytes([
        bytes.get(0).copied().unwrap_or(0),
        bytes.get(1).copied().unwrap_or(0),
        bytes.get(2).copied().unwrap_or(0),
        bytes.get(3).copied().unwrap_or(0),
    ]);

    eprintln!("[VARINT30] ch={:#04x}, extra={}, result={}", ch, extra, result);
    Ok(result as i64)
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

pub const NDX_DONE: i32 = -1;
pub const NDX_FLIST_EOF: i32 = -2;

pub struct NdxState {
    prev_positive: i32,
    prev_negative: i32,
}

impl NdxState {
    pub fn new() -> Self {
        Self {
            prev_positive: -1,
            prev_negative: -1,
        }
    }
}

pub fn write_ndx<W: Write>(writer: &mut W, ndx: i32, state: &mut NdxState, protocol_version: i32) -> Result<()> {
    if protocol_version < 30 {
        writer.write_i32::<LittleEndian>(ndx)?;
        return Ok(());
    }

    let diff: i32;
    let cnt: u8;

    if ndx >= 0 {
        diff = ndx - state.prev_positive;
        state.prev_positive = ndx;
    } else if ndx == NDX_DONE {
        writer.write_u8(0)?;
        return Ok(());
    } else {
        diff = state.prev_negative - ndx;
        state.prev_negative = ndx;
        if diff < 1 || diff > 0xFE {
            writer.write_u8(0xFF)?;
            writer.write_i32::<LittleEndian>(ndx)?;
            return Ok(());
        }
        cnt = diff as u8;
        writer.write_u8(cnt)?;
        return Ok(());
    }

    if diff < 0xFE && diff > 0 {
        cnt = diff as u8;
        writer.write_u8(cnt)?;
    } else if diff < 0 || diff > 0x7FFF {
        writer.write_u8(0xFE)?;
        writer.write_i32::<LittleEndian>(ndx | 0x80000000u32 as i32)?;
    } else {
        writer.write_u8(0xFE)?;
        writer.write_u16::<LittleEndian>(diff as u16)?;
    }

    Ok(())
}

pub fn read_ndx<R: Read>(reader: &mut R, state: &mut NdxState, protocol_version: i32) -> Result<i32> {
    if protocol_version < 30 {
        return Ok(reader.read_i32::<LittleEndian>()?);
    }

    let mut b = reader.read_u8()?;
    eprintln!("[NDX] Read first byte: 0x{:02x}", b);

    let is_negative = if b == 0xFF {
        eprintln!("[NDX] b==0xFF, reading next byte...");
        b = reader.read_u8()?;
        eprintln!("[NDX] Read second byte: 0x{:02x}", b);
        true
    } else if b == 0 {
        eprintln!("[NDX] b==0, returning NDX_DONE");
        return Ok(NDX_DONE);
    } else {
        false
    };

    let num = if b == 0xFE {
        eprintln!("[NDX] b==0xFE, reading 2 more bytes...");
        let b0 = reader.read_u8()?;
        let b1 = reader.read_u8()?;
        eprintln!("[NDX] b0=0x{:02x}, b1=0x{:02x}", b0, b1);

        if (b0 & 0x80) != 0 {
            eprintln!("[NDX] b0 & 0x80, reading 2 more bytes (4-byte mode)...");
            let b3 = b0 & !0x80;
            let b2 = reader.read_u8()?;
            let b3_new = reader.read_u8()?;
            eprintln!("[NDX] b2=0x{:02x}, b3_new=0x{:02x}", b2, b3_new);

            let value = (b1 as i32) | ((b2 as i32) << 8) | ((b3_new as i32) << 16) | ((b3 as i32) << 24);
            eprintln!("[NDX] 4-byte value: {}", value);
            value
        } else {
            let value = ((b0 as i32) << 8) + (b1 as i32);
            let prev = if is_negative { state.prev_negative } else { state.prev_positive };
            let result = value + prev;
            eprintln!("[NDX] 2-byte value: {}, prev: {}, result: {}", value, prev, result);
            result
        }
    } else {
        let prev = if is_negative { state.prev_negative } else { state.prev_positive };
        let result = (b as i32) + prev;
        eprintln!("[NDX] Single byte: {}, prev: {}, result: {}", b, prev, result);
        result
    };

    if is_negative {
        state.prev_negative = num;
        eprintln!("[NDX] Final (negative): -{}", -num);
        Ok(-num)
    } else {
        state.prev_positive = num;
        eprintln!("[NDX] Final (positive): {}", num);
        Ok(num)
    }
}

pub fn read_ndx_and_attrs<R: Read>(
    reader: &mut R,
    state: &mut NdxState,
    protocol_version: i32,
) -> Result<(i32, u16, Option<u8>, Option<String>)> {
    let ndx = read_ndx(reader, state, protocol_version)?;

    if ndx == NDX_DONE {
        return Ok((ndx, 0, None, None));
    }

    let iflags = if protocol_version >= 29 {
        read_shortint(reader)?
    } else {
        0
    };

    let fnamecmp_type = if (iflags & ITEM_BASIS_TYPE_FOLLOWS) != 0 {
        Some(reader.read_u8()?)
    } else {
        None
    };

    let xname = if (iflags & ITEM_XNAME_FOLLOWS) != 0 {
        Some(read_vstring(reader)?)
    } else {
        None
    };

    Ok((ndx, iflags, fnamecmp_type, xname))
}
