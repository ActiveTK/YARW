use std::io::{Read, Write};
use std::collections::VecDeque;
use byteorder::{ReadBytesExt, WriteBytesExt, LittleEndian};
use crate::error::{Result, RsyncError};

const MPLEX_BASE: u8 = 7;
const MSG_DATA: u8 = 0;
const MAX_MPLEX_DATA: usize = 0xFFFFFF;

pub struct MultiplexIO<T> {
    inner: T,
    read_buffer: VecDeque<u8>,
}

impl<T> MultiplexIO<T> {
    pub fn new(inner: T) -> Self {
        Self {
            inner,
            read_buffer: VecDeque::new(),
        }
    }

    pub fn into_inner(self) -> T {
        self.inner
    }
}

impl<T: Read> MultiplexIO<T> {
    fn read_packet(&mut self) -> Result<()> {
        eprintln!("[MPLEX] About to read header... (buffer has {} bytes)", self.read_buffer.len());

        let mut header_bytes = [0u8; 4];
        let mut total_read = 0;
        while total_read < 4 {
            match self.inner.read(&mut header_bytes[total_read..]) {
                Ok(0) => {
                    eprintln!("[MPLEX] EOF encountered after reading {} bytes", total_read);
                    return Err(RsyncError::Io(std::io::Error::new(
                        std::io::ErrorKind::UnexpectedEof,
                        "failed to fill whole buffer"
                    )));
                }
                Ok(n) => {
                    eprintln!("[MPLEX] Read {} bytes of header (total: {}/4)", n, total_read + n);
                    total_read += n;
                }
                Err(e) => {
                    eprintln!("[MPLEX] Failed to read header: {}", e);
                    return Err(RsyncError::Io(e));
                }
            }
        }
        eprintln!("[MPLEX] Read header bytes: {:02x} {:02x} {:02x} {:02x}",
            header_bytes[0], header_bytes[1], header_bytes[2], header_bytes[3]);

        let header = u32::from_le_bytes(header_bytes);

        let tag = (header >> 24) as u8;
        let length = (header & 0x00FFFFFF) as usize;

        eprintln!("[MPLEX] Read header: tag={}, length={}", tag, length);

        let msg_code = tag.wrapping_sub(MPLEX_BASE);

        if msg_code != MSG_DATA {
            let mut msg_data = vec![0u8; length];
            self.inner.read_exact(&mut msg_data)?;

            let msg_str = String::from_utf8_lossy(&msg_data);
            eprintln!("[MPLEX] Non-data message (code {}): {}", msg_code, msg_str);

            if msg_code >= 1 && msg_code <= 3 {
                eprintln!("Remote error (code {}): {}", msg_code, msg_str);
                return Err(RsyncError::RemoteExec(format!("Server error: {}", msg_str)));
            }

            return Ok(());
        }

        eprintln!("[MPLEX] Reading {} bytes of data", length);
        let mut data = vec![0u8; length];
        self.inner.read_exact(&mut data)?;

        let dump_len = length.min(100);
        eprintln!("[MPLEX] Hex dump of first {} bytes:", dump_len);
        for (i, chunk) in data[..dump_len].chunks(16).enumerate() {
            eprint!("  {:04x}: ", i * 16);
            for byte in chunk {
                eprint!("{:02x} ", byte);
            }
            eprintln!();
        }

        self.read_buffer.extend(data);
        eprintln!("[MPLEX] Buffer now has {} bytes", self.read_buffer.len());

        Ok(())
    }
}

impl<T: Read> Read for MultiplexIO<T> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        if buf.is_empty() {
            return Ok(0);
        }

        eprintln!("[MPLEX-READ] Request {} bytes, buffer has {}", buf.len(), self.read_buffer.len());

        while self.read_buffer.is_empty() {
            match self.read_packet() {
                Ok(()) => {},
                Err(RsyncError::Io(e)) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                    eprintln!("[MPLEX-READ] Hit EOF, returning 0");
                    return Ok(0);
                },
                Err(e) => return Err(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())),
            }
        }

        let len = buf.len().min(self.read_buffer.len());
        for i in 0..len {
            buf[i] = self.read_buffer.pop_front().unwrap();
        }
        eprintln!("[MPLEX-READ] Returning {} bytes, buffer now has {}", len, self.read_buffer.len());
        Ok(len)
    }
}

impl<T: Write> Write for MultiplexIO<T> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        if buf.is_empty() {
            return Ok(0);
        }

        let len = buf.len();
        let tag = MPLEX_BASE + MSG_DATA;
        let header = ((tag as u32) << 24) | (len as u32 & 0x00FFFFFF);

        eprintln!("[MPLEX-WRITE] Sending multiplexed data: tag={}, length={}", tag, len);

        self.inner.write_all(&header.to_le_bytes())?;
        self.inner.write_all(buf)?;

        Ok(len)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.inner.flush()
    }
}
