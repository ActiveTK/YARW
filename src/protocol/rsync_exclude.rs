use std::io::{Read, Write};
use crate::error::Result;
use byteorder::{ReadBytesExt, WriteBytesExt, LittleEndian};

pub struct ExcludeList {
    pub rules: Vec<String>,
}

impl ExcludeList {
    pub fn new() -> Self {
        Self { rules: Vec::new() }
    }

    pub fn send<W: Write>(&self, writer: &mut W) -> Result<()> {
        eprintln!("[EXCLUDE] Sending {} exclusion rules", self.rules.len());
        for rule in &self.rules {
            let rule_bytes = rule.as_bytes();
            eprintln!("[EXCLUDE] Sending rule (len={}): {}", rule_bytes.len(), rule);
            writer.write_i32::<LittleEndian>(rule_bytes.len() as i32)?;
            writer.write_all(rule_bytes)?;
        }
        eprintln!("[EXCLUDE] Sending terminator (0)");
        writer.write_i32::<LittleEndian>(0)?;
        eprintln!("[EXCLUDE] Exclude list send complete");
        Ok(())
    }

    pub fn recv<R: Read>(reader: &mut R) -> Result<Self> {
        let mut rules = Vec::new();
        loop {
            let len = reader.read_i32::<LittleEndian>()?;
            if len == 0 {
                break;
            }
            if len < 0 || len > 1048576 {
                break;
            }
            let mut rule_bytes = vec![0u8; len as usize];
            reader.read_exact(&mut rule_bytes)?;
            if let Ok(rule) = String::from_utf8(rule_bytes) {
                rules.push(rule);
            }
        }
        Ok(Self { rules })
    }
}
