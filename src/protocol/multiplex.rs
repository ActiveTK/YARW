use std::io::{Read, Write};
use std::collections::VecDeque;
use byteorder::{ReadBytesExt, WriteBytesExt, BigEndian};
use crate::error::{Result, RsyncError};

const MPLEX_BASE: u8 = 7;
const MSG_DATA: u8 = 0;

pub struct MultiplexReader<R: Read> {
    inner: R,
    buffer: VecDeque<u8>,
}

impl<R: Read> MultiplexReader<R> {
    pub fn new(inner: R) -> Self {
        Self {
            inner,
            buffer: VecDeque::new(),
        }
    }

    fn read_packet(&mut self) -> Result<()> {
        let header = self.inner.read_u32::<BigEndian>()?;

        let tag = (header >> 24) as u8;
        let length = (header & 0x00FFFFFF) as usize;

        let msg_code = tag.wrapping_sub(MPLEX_BASE);

        if msg_code != MSG_DATA {
            let mut msg_data = vec![0u8; length];
            self.inner.read_exact(&mut msg_data)?;

            if msg_code >= 1 && msg_code <= 3 {
                eprintln!("Remote error: {}", String::from_utf8_lossy(&msg_data));
            }

            return Ok(());
        }

        let mut data = vec![0u8; length];
        self.inner.read_exact(&mut data)?;
        self.buffer.extend(data);

        Ok(())
    }
}

impl<R: Read> Read for MultiplexReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        if buf.is_empty() {
            return Ok(0);
        }

        while self.buffer.is_empty() {
            match self.read_packet() {
                Ok(()) => {},
                Err(RsyncError::Io(e)) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                    return Ok(0);
                },
                Err(e) => return Err(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())),
            }
        }

        let len = buf.len().min(self.buffer.len());
        for i in 0..len {
            buf[i] = self.buffer.pop_front().unwrap();
        }
        Ok(len)
    }
}

pub struct MultiplexWriter<W: Write> {
    inner: W,
}

impl<W: Write> MultiplexWriter<W> {
    pub fn new(inner: W) -> Self {
        Self { inner }
    }
}

impl<W: Write> Write for MultiplexWriter<W> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        if buf.is_empty() {
            return Ok(0);
        }

        let tag = MPLEX_BASE + MSG_DATA;
        let header = ((tag as u32) << 24) | (buf.len() as u32 & 0x00FFFFFF);

        self.inner.write_u32::<BigEndian>(header)?;
        self.inner.write_all(buf)?;

        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.inner.flush()
    }
}
