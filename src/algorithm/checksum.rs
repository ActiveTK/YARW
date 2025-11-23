use crate::options::ChecksumAlgorithm;
use blake2::Blake2b512;
use digest::Digest;
use md4::Md4 as Md4Hasher;
use md5::Md5 as Md5Hasher;



#[derive(Debug, Clone)]
pub struct RollingChecksum {

    a: u16,

    b: u16,

    block_size: usize,
}

impl RollingChecksum {

    pub fn new(data: &[u8]) -> Self {
        let mut checksum = Self {
            a: 0,
            b: 0,
            block_size: data.len(),
        };
        checksum.update(data);
        checksum
    }


    fn update(&mut self, data: &[u8]) {
        self.a = 0;
        self.b = 0;

        for (i, &byte) in data.iter().enumerate() {
            self.a = self.a.wrapping_add(byte as u16);
            self.b = self
                .b
                .wrapping_add(((data.len() - i) as u16).wrapping_mul(byte as u16));
        }
    }



    pub fn roll(&mut self, old_byte: u8, new_byte: u8) {

        self.a = self
            .a
            .wrapping_sub(old_byte as u16)
            .wrapping_add(new_byte as u16);


        self.b = self
            .b
            .wrapping_sub((self.block_size as u16).wrapping_mul(old_byte as u16))
            .wrapping_add(self.a);
    }


    pub fn checksum(&self) -> u32 {
        ((self.b as u32) << 16) | (self.a as u32)
    }


    #[allow(dead_code)]
    pub fn block_size(&self) -> usize {
        self.block_size
    }
}


#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StrongChecksum {
    Md4([u8; 16]),
    Md5([u8; 16]),
    Blake2([u8; 64]),
}

impl StrongChecksum {

    pub fn as_bytes(&self) -> &[u8] {
        match self {
            StrongChecksum::Md4(bytes) => bytes,
            StrongChecksum::Md5(bytes) => bytes,
            StrongChecksum::Blake2(bytes) => bytes,
        }
    }
}


pub fn compute_strong_checksum(data: &[u8], algorithm: &ChecksumAlgorithm) -> StrongChecksum {
    match algorithm {
        ChecksumAlgorithm::Md4 => {
            let mut hasher = Md4Hasher::new();
            hasher.update(data);
            let result = hasher.finalize();
            let mut bytes = [0u8; 16];
            bytes.copy_from_slice(&result);
            StrongChecksum::Md4(bytes)
        }
        ChecksumAlgorithm::Md5 => {
            let mut hasher = Md5Hasher::new();
            hasher.update(data);
            let result = hasher.finalize();
            let mut bytes = [0u8; 16];
            bytes.copy_from_slice(&result);
            StrongChecksum::Md5(bytes)
        }
        ChecksumAlgorithm::Blake2 => {
            let mut hasher = Blake2b512::new();
            hasher.update(data);
            let result = hasher.finalize();
            let mut bytes = [0u8; 64];
            bytes.copy_from_slice(&result);
            StrongChecksum::Blake2(bytes)
        }
        ChecksumAlgorithm::Xxh128 => {


            let mut hasher = Md5Hasher::new();
            hasher.update(data);
            let result = hasher.finalize();
            let mut bytes = [0u8; 16];
            bytes.copy_from_slice(&result);
            StrongChecksum::Md5(bytes)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rolling_checksum_basic() {
        let data = b"hello world";
        let checksum = RollingChecksum::new(data);


        assert_ne!(checksum.checksum(), 0);
        assert_eq!(checksum.block_size(), data.len());
    }

    #[test]
    fn test_rolling_checksum_roll() {

        let data1 = b"abc";
        let mut checksum1 = RollingChecksum::new(data1);

        let data2 = b"bcd";
        let checksum2 = RollingChecksum::new(data2);


        checksum1.roll(b'a', b'd');


        assert_eq!(checksum1.checksum(), checksum2.checksum());
    }

    #[test]
    fn test_rolling_checksum_sliding_window() {
        let data = b"abcdefgh";
        let window_size = 4;


        let mut rolling = RollingChecksum::new(&data[0..window_size]);
        let first_checksum = rolling.checksum();


        rolling.roll(data[0], data[4]);
        let second_checksum = rolling.checksum();


        let direct = RollingChecksum::new(&data[1..5]);
        assert_eq!(second_checksum, direct.checksum());


        assert_ne!(first_checksum, second_checksum);
    }

    #[test]
    fn test_strong_checksum_md5() {
        let data = b"test data";
        let checksum = compute_strong_checksum(data, &ChecksumAlgorithm::Md5);

        match checksum {
            StrongChecksum::Md5(bytes) => {
                assert_eq!(bytes.len(), 16);

                let checksum2 = compute_strong_checksum(data, &ChecksumAlgorithm::Md5);
                assert_eq!(checksum, checksum2);
            }
            _ => panic!("Expected Md5 checksum"),
        }
    }

    #[test]
    fn test_strong_checksum_different_algorithms() {
        let data = b"test data";

        let md4 = compute_strong_checksum(data, &ChecksumAlgorithm::Md4);
        let md5 = compute_strong_checksum(data, &ChecksumAlgorithm::Md5);
        let blake2 = compute_strong_checksum(data, &ChecksumAlgorithm::Blake2);


        assert_ne!(md4.as_bytes(), md5.as_bytes());
        assert_ne!(md5.as_bytes(), blake2.as_bytes());
    }

    #[test]
    fn test_strong_checksum_deterministic() {
        let data = b"deterministic test";


        let checksum1 = compute_strong_checksum(data, &ChecksumAlgorithm::Md5);
        let checksum2 = compute_strong_checksum(data, &ChecksumAlgorithm::Md5);

        assert_eq!(checksum1, checksum2);
    }
}
