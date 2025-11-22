use std::io::{Read, Write};
use byteorder::{ReadBytesExt, WriteBytesExt, BigEndian, LittleEndian};
use crate::error::{Result, RsyncError};

/// rsyncプロトコルストリーム
///
/// rsyncプロトコルで定義されているデータ型を読み書きするためのラッパー
pub struct ProtocolStream<S: Read + Write> {
    stream: S,
    /// プロトコルバージョンによってエンディアンが異なる場合があるため保持
    #[allow(dead_code)]
    protocol_version: i32,
}

impl<S: Read + Write + ReadBytesExt + WriteBytesExt> ProtocolStream<S> {
    /// 新しいProtocolStreamを作成
    pub fn new(stream: S, protocol_version: i32) -> Self {
        Self { stream, protocol_version }
    }

    // --- 基本的なデータ型の読み書き ---

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

    // --- rsync独自のデータ型の読み書き ---

    /// 可変長整数 (Variable-length integer) を読み込む
    ///
    /// rsyncでは、多くの場合、数値を可変長でエンコードして帯域幅を節約する。
    /// プロトコルバージョン27以降では、i32/i64をそのまま送るのではなく、
    /// この形式が使われることが多い。
    pub fn read_varint(&mut self) -> Result<i64> {
        let first_byte = self.read_i8()? as u8;

        match first_byte {
            // 1バイトで表現
            0..=250 => Ok(first_byte as i64),
            // 2バイトで表現
            251 => Ok(self.stream.read_i16::<BigEndian>()? as i64),
            // 4バイトで表現
            252 => Ok(self.stream.read_i32::<BigEndian>()? as i64),
            // 8バイトで表現
            253 => Ok(self.stream.read_i64::<BigEndian>()? as i64),
            // 負の1バイト
            254 => Ok(self.read_i8()? as i64),
            // 負の2バイト
            255 => Ok(self.stream.read_i16::<BigEndian>()? as i64),
            // 上記以外は予約済み or エラー
        }
    }

    /// 可変長整数を書き込む
    pub fn write_varint(&mut self, val: i64) -> Result<()> {
        match val {
            // 1バイトで表現（正の値 0-250）
            0..=250 => {
                self.stream.write_u8(val as u8)?;
                Ok(())
            }
            // 負の1バイト整数（-128 to -1）
            -128..=-1 => {
                self.stream.write_u8(254)?;
                self.write_i8(val as i8)
            }
            // 2バイト整数（正）
            251..=32767 => {
                self.stream.write_u8(251)?;
                self.stream.write_i16::<BigEndian>(val as i16)?;
                Ok(())
            }
            // 2バイト整数（負）
            -32768..=-129 => {
                self.stream.write_u8(255)?;
                self.stream.write_i16::<BigEndian>(val as i16)?;
                Ok(())
            }
            // 4バイト整数
            -2147483648..=2147483647 => {
                self.stream.write_u8(252)?;
                self.stream.write_i32::<BigEndian>(val as i32)?;
                Ok(())
            }
            // 8バイト整数（残り全て）
            _ => {
                self.stream.write_u8(253)?;
                self.stream.write_i64::<BigEndian>(val)?;
                Ok(())
            }
        }
    }

    /// null終端文字列を読み込む
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

    /// null終端文字列を書き込む
    pub fn write_string(&mut self, s: &str) -> Result<()> {
        self.write_all(s.as_bytes())?;
        self.write_i8(0)?; // null終端
        Ok(())
    }

    /// ストリームをフラッシュ
    pub fn flush(&mut self) -> Result<()> {
        Ok(self.stream.flush()?)
    }

    /// 内部ストリームへの参照を取得
    #[allow(dead_code)]
    pub fn get_ref(&self) -> &S {
        &self.stream
    }

    /// 内部ストリームへの可変参照を取得
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

        // 次の読み込みは "world" になるはずだが、read_stringは1文字ずつ読むので...
        // ストリームの位置を確認
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
            0, 1, 100, 250,          // 1バイト整数（正）
            -1, -50, -128,            // 1バイト整数（負）
            251, 1000, 32767,         // 2バイト整数（正）
            -129, -1000, -32768,      // 2バイト整数（負）
            32768, 1000000,           // 4バイト整数（正）
            -32769, -1000000,         // 4バイト整数（負）
            2147483648,               // 8バイト整数（正）
            -2147483649,              // 8バイト整数（負）
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

        // 1バイト値は1バイトでエンコードされる
        stream.write_varint(100)?;
        assert_eq!(stream.get_ref().get_ref().len(), 1);

        // 2バイト値は3バイトでエンコードされる（タグ1バイト + 値2バイト）
        stream.get_mut().get_mut().clear();
        stream.get_mut().set_position(0);
        stream.write_varint(1000)?;
        assert_eq!(stream.get_ref().get_ref().len(), 3);

        Ok(())
    }
}
