use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use crate::error::{Result, RsyncError};
use std::io::Cursor;




pub struct AsyncProtocolStream<S> {
    stream: S,
    #[allow(dead_code)]
    protocol_version: i32,
}

impl<S: AsyncRead + AsyncWrite + Unpin> AsyncProtocolStream<S> {

    pub fn new(stream: S, protocol_version: i32) -> Self {
        Self { stream, protocol_version }
    }



    pub async fn read_i8(&mut self) -> Result<i8> {
        Ok(self.stream.read_i8().await?)
    }

    pub async fn write_i8(&mut self, val: i8) -> Result<()> {
        Ok(self.stream.write_i8(val).await?)
    }

    pub async fn read_i32(&mut self) -> Result<i32> {
        let mut buf = [0u8; 4];
        self.stream.read_exact(&mut buf).await?;
        let mut cursor = Cursor::new(buf);
        Ok(ReadBytesExt::read_i32::<LittleEndian>(&mut cursor)?)
    }

    pub async fn write_i32(&mut self, val: i32) -> Result<()> {
        let mut buf = Vec::new();
        WriteBytesExt::write_i32::<LittleEndian>(&mut buf, val)?;
        self.stream.write_all(&buf).await?;
        Ok(())
    }

    pub async fn read_u8(&mut self) -> Result<u8> {
        Ok(self.stream.read_u8().await?)
    }

    pub async fn write_u8(&mut self, val: u8) -> Result<()> {
        Ok(self.stream.write_u8(val).await?)
    }



    pub async fn read_varint(&mut self) -> Result<i64> {
        let first = self.read_u8().await?;

        match first {
            0..=250 => Ok(first as i64),
            251 => {
                let mut buf = [0u8; 2];
                self.stream.read_exact(&mut buf).await?;
                let mut cursor = Cursor::new(buf);
                Ok(ReadBytesExt::read_i16::<LittleEndian>(&mut cursor)? as i64)
            }
            252 => {
                let mut buf = [0u8; 4];
                self.stream.read_exact(&mut buf).await?;
                let mut cursor = Cursor::new(buf);
                Ok(ReadBytesExt::read_i32::<LittleEndian>(&mut cursor)? as i64)
            }
            253 => {
                let mut buf = [0u8; 8];
                self.stream.read_exact(&mut buf).await?;
                let mut cursor = Cursor::new(buf);
                Ok(ReadBytesExt::read_i64::<LittleEndian>(&mut cursor)?)
            }
            254 => Ok(self.read_i8().await? as i64),
            255 => Err(RsyncError::Other("Invalid varint tag 255".to_string())),
        }
    }

    pub async fn write_varint(&mut self, val: i64) -> Result<()> {
        if val >= 0 && val <= 250 {
            self.write_u8(val as u8).await?;
        } else if val >= -128 && val <= -1 {
            self.write_u8(254).await?;
            self.write_i8(val as i8).await?;
        } else if val >= 251 && val <= 32767 {
            self.write_u8(251).await?;
            let mut buf = Vec::new();
            WriteBytesExt::write_i16::<LittleEndian>(&mut buf, val as i16)?;
            self.stream.write_all(&buf).await?;
        } else if (val >= 32768 && val <= i32::MAX as i64) || (val >= i32::MIN as i64 && val <= -129) {
            self.write_u8(252).await?;
            let mut buf = Vec::new();
            WriteBytesExt::write_i32::<LittleEndian>(&mut buf, val as i32)?;
            self.stream.write_all(&buf).await?;
        } else {
            self.write_u8(253).await?;
            let mut buf = Vec::new();
            WriteBytesExt::write_i64::<LittleEndian>(&mut buf, val)?;
            self.stream.write_all(&buf).await?;
        }
        Ok(())
    }



    pub async fn read_string(&mut self, max_len: usize) -> Result<String> {
        let mut bytes = Vec::new();
        loop {
            let byte = self.read_u8().await?;
            if byte == 0 {
                break;
            }
            bytes.push(byte);
            if bytes.len() > max_len {
                return Err(RsyncError::Other(format!(
                    "String too long (max: {})",
                    max_len
                )));
            }
        }
        Ok(String::from_utf8(bytes)?)
    }

    pub async fn write_string(&mut self, s: &str) -> Result<()> {
        self.stream.write_all(s.as_bytes()).await?;
        self.write_u8(0).await?;
        Ok(())
    }



    pub async fn read_all(&mut self, buf: &mut [u8]) -> Result<()> {
        self.stream.read_exact(buf).await?;
        Ok(())
    }

    pub async fn write_all(&mut self, buf: &[u8]) -> Result<()> {
        self.stream.write_all(buf).await?;
        Ok(())
    }

    pub async fn flush(&mut self) -> Result<()> {
        self.stream.flush().await?;
        Ok(())
    }
}
