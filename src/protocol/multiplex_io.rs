use std::io::{Read, Write};
use std::collections::VecDeque;
use byteorder::{ReadBytesExt, WriteBytesExt, BigEndian};
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
        eprintln!("[MPLEX] About to read header...");

        let mut header_bytes = [0u8; 4];
        match self.inner.read_exact(&mut header_bytes) {
            Ok(()) => {
                eprintln!("[MPLEX] Read header bytes: {:02x} {:02x} {:02x} {:02x}",
                    header_bytes[0], header_bytes[1], header_bytes[2], header_bytes[3]);
            }
            Err(e) => {
                eprintln!("[MPLEX] Failed to read header: {}", e);
                return Err(RsyncError::Io(e));
            }
        }

        let header = u32::from_be_bytes(header_bytes);

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

        while self.read_buffer.is_empty() {
            match self.read_packet() {
                Ok(()) => {},
                Err(RsyncError::Io(e)) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                    return Ok(0);
                },
                Err(e) => return Err(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())),
            }
        }

        let len = buf.len().min(self.read_buffer.len());
        for i in 0..len {
            buf[i] = self.read_buffer.pop_front().unwrap();
        }
        Ok(len)
    }
}

impl<T: Write> Write for MultiplexIO<T> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.inner.write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.inner.flush()
    }
}
