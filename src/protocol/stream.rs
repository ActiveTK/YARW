use std::io::{Read, Write};
use byteorder::{ReadBytesExt, WriteBytesExt, BigEndian, LittleEndian};
use crate::error::{Result, RsyncError};




pub struct ProtocolStream<S: Read + Write> {
    stream: S,

    #[allow(dead_code)]
    protocol_version: i32,
}

impl<S: Read + Write + ReadBytesExt + WriteBytesExt> ProtocolStream<S> {

    pub fn new(stream: S, protocol_version: i32) -> Self {
        Self { stream, protocol_version }
    }



    pub fn read_i8(&mut self) -> Result<i8> {
        Ok(self.stream.read_i8()?)
    }

    pub fn write_i8(&mut self, val: i8) -> Result<()> {
        Ok(self.stream.write_i8(val)?)
    }

    pub fn read_i32(&mut self) -> Result<i32> {
        Ok(self.stream.read_i32::<LittleEndian>()?)
    }

    pub fn write_i32(&mut self, val: i32) -> Result<()> {
        Ok(self.stream.write_i32::<LittleEndian>(val)?)
    }

    #[allow(dead_code)]
    pub fn read_i64(&mut self) -> Result<i64> {
        Ok(self.stream.read_i64::<LittleEndian>()?)
    }

    #[allow(dead_code)]
    pub fn write_i64(&mut self, val: i64) -> Result<()> {
        Ok(self.stream.write_i64::<LittleEndian>(val)?)
    }

    pub fn read_all(&mut self, buf: &mut [u8]) -> Result<()> {
        Ok(self.stream.read_exact(buf)?)
    }

    pub fn write_all(&mut self, buf: &[u8]) -> Result<()> {
        Ok(self.stream.write_all(buf)?)
    }








    pub fn read_varint(&mut self) -> Result<i64> {
        let first_byte = self.read_i8()? as u8;

        match first_byte {

            0..=250 => Ok(first_byte as i64),

            251 => Ok(self.stream.read_i16::<BigEndian>()? as i64),

            252 => Ok(self.stream.read_i32::<BigEndian>()? as i64),

            253 => Ok(self.stream.read_i64::<BigEndian>()? as i64),

            254 => Ok(self.read_i8()? as i64),

            255 => Ok(self.stream.read_i16::<BigEndian>()? as i64),

        }
    }


    pub fn write_varint(&mut self, val: i64) -> Result<()> {
        match val {

            0..=250 => {
                self.stream.write_u8(val as u8)?;
                Ok(())
            }

            -128..=-1 => {
                self.stream.write_u8(254)?;
                self.write_i8(val as i8)
            }

            251..=32767 => {
                self.stream.write_u8(251)?;
                self.stream.write_i16::<BigEndian>(val as i16)?;
                Ok(())
            }

            -32768..=-129 => {
                self.stream.write_u8(255)?;
                self.stream.write_i16::<BigEndian>(val as i16)?;
                Ok(())
            }

            -2147483648..=2147483647 => {
                self.stream.write_u8(252)?;
                self.stream.write_i32::<BigEndian>(val as i32)?;
                Ok(())
            }

            _ => {
                self.stream.write_u8(253)?;
                self.stream.write_i64::<BigEndian>(val)?;
                Ok(())
            }
        }
    }


    pub fn read_string(&mut self, max_len: usize) -> Result<String> {
        let mut bytes = Vec::new();
        let mut byte = [0u8; 1];

        loop {
            self.read_all(&mut byte)?;
            if byte[0] == 0 {
                break;
            }
            bytes.push(byte[0]);

            if bytes.len() > max_len {
                return Err(RsyncError::Other("String length limit exceeded".to_string()));
            }
        }

        Ok(String::from_utf8(bytes)?)
    }


    pub fn write_string(&mut self, s: &str) -> Result<()> {
        self.write_all(s.as_bytes())?;
        self.write_i8(0)?;
        Ok(())
    }


    pub fn flush(&mut self) -> Result<()> {
        Ok(self.stream.flush()?)
    }


    #[allow(dead_code)]
    pub fn get_ref(&self) -> &S {
        &self.stream
    }


    #[allow(dead_code)]
    pub fn get_mut(&mut self) -> &mut S {
        &mut self.stream
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_read_write_i32() -> Result<()> {
        let mut buffer = Cursor::new(Vec::new());
        let mut stream = ProtocolStream::new(&mut buffer, 31);

        stream.write_i32(123456)?;
        stream.get_mut().set_position(0);
        let val = stream.read_i32()?;

        assert_eq!(val, 123456);
        Ok(())
    }

    #[test]
    fn test_read_string() -> Result<()> {
        let data = b"hello\0world".to_vec();
        let mut buffer = Cursor::new(data);
        let mut stream = ProtocolStream::new(&mut buffer, 31);

        let s = stream.read_string(100)?;
        assert_eq!(s, "hello");



        assert_eq!(stream.get_ref().position(), 6);

        Ok(())
    }

    #[test]
    fn test_write_string() -> Result<()> {
        let mut buffer = Cursor::new(Vec::new());
        let mut stream = ProtocolStream::new(&mut buffer, 31);

        stream.write_string("test")?;
        stream.get_mut().set_position(0);

        let s = stream.read_string(100)?;
        assert_eq!(s, "test");

        Ok(())
    }

    #[test]
    fn test_varint_round_trip() -> Result<()> {
        let test_values = vec![
            0, 1, 100, 250,
            -1, -50, -128,
            251, 1000, 32767,
            -129, -1000, -32768,
            32768, 1000000,
            -32769, -1000000,
            2147483648,
            -2147483649,
        ];

        for &val in &test_values {
            let mut buffer = Cursor::new(Vec::new());
            let mut stream = ProtocolStream::new(&mut buffer, 31);

            stream.write_varint(val)?;
            stream.get_mut().set_position(0);
            let read_val = stream.read_varint()?;

            assert_eq!(val, read_val, "Failed for value: {}", val);
        }

        Ok(())
    }

    #[test]
    fn test_varint_encoding_size() -> Result<()> {
        let mut buffer = Cursor::new(Vec::new());
        let mut stream = ProtocolStream::new(&mut buffer, 31);


        stream.write_varint(100)?;
        assert_eq!(stream.get_ref().get_ref().len(), 1);


        stream.get_mut().get_mut().clear();
        stream.get_mut().set_position(0);
        stream.write_varint(1000)?;
        assert_eq!(stream.get_ref().get_ref().len(), 3);

        Ok(())
    }
}
