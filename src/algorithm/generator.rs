use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;
use crate::error::Result;
use crate::options::ChecksumAlgorithm;
use crate::algorithm::checksum::{RollingChecksum, StrongChecksum, compute_strong_checksum};
use crate::filesystem::buffer_optimizer::BufferOptimizer;
use crate::algorithm::parallel_checksum::ParallelChecksumEngine;


#[derive(Debug, Clone)]
pub struct BlockChecksum {

    pub index: u32,

    pub weak: u32,

    pub strong: StrongChecksum,
}


pub struct Generator {

    block_size: usize,

    checksum_algorithm: ChecksumAlgorithm,
}

impl Generator {

    pub fn new(block_size: usize, checksum_algorithm: ChecksumAlgorithm) -> Self {
        Self {
            block_size,
            checksum_algorithm,
        }
    }



    pub fn calculate_block_size(file_size: u64) -> usize {
        let optimizer = BufferOptimizer::new();
        optimizer.optimal_buffer_size(file_size)
    }


    pub fn generate_checksums(&self, file_path: &Path) -> Result<Vec<BlockChecksum>> {
        let metadata = std::fs::metadata(file_path)?;
        let file_size = metadata.len();

        const PARALLEL_THRESHOLD: u64 = 1024 * 1024;

        if file_size >= PARALLEL_THRESHOLD {
            let data = std::fs::read(file_path)?;
            let parallel_engine = ParallelChecksumEngine::new(self.checksum_algorithm);
            Ok(parallel_engine.compute_block_checksums_parallel(&data, self.block_size))
        } else {
            let optimizer = BufferOptimizer::new();
            let reader_buffer_size = optimizer.optimal_buffer_for_file(file_path);
            let file = File::open(file_path)?;
            let mut reader = BufReader::with_capacity(reader_buffer_size, file);
            let mut checksums = Vec::new();
            let mut buffer = vec![0u8; self.block_size];
            let mut index = 0u32;

            loop {
                let bytes_read = reader.read(&mut buffer)?;
                if bytes_read == 0 {
                    break;
                }

                let block = &buffer[..bytes_read];

                let rolling = RollingChecksum::new(block);
                let weak = rolling.checksum();

                let strong = compute_strong_checksum(block, &self.checksum_algorithm);

                checksums.push(BlockChecksum {
                    index,
                    weak,
                    strong,
                });

                index += 1;
            }

            Ok(checksums)
        }
    }


    #[allow(dead_code)]
    pub fn block_size(&self) -> usize {
        self.block_size
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs;
    use std::io::Write;

    #[test]
    fn test_calculate_block_size() {

        assert_eq!(Generator::calculate_block_size(0), 700);
        assert_eq!(Generator::calculate_block_size(1024), 700);


        let size_1mb = Generator::calculate_block_size(1024 * 1024);
        assert!(size_1mb >= 700 && size_1mb <= 128 * 1024);
        assert_eq!(size_1mb, 1024);


        let size_100mb = Generator::calculate_block_size(100 * 1024 * 1024);
        assert!(size_100mb >= 700 && size_100mb <= 128 * 1024);


        let size_10gb = Generator::calculate_block_size(10u64 * 1024 * 1024 * 1024);
        assert!(size_10gb >= 700 && size_10gb <= 128 * 1024);



        let size_100gb = Generator::calculate_block_size(100u64 * 1024 * 1024 * 1024);
        assert_eq!(size_100gb, 128 * 1024);
    }

    #[test]
    fn test_generate_checksums_small_file() -> Result<()> {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");


        let content = b"Hello, rsync!";
        fs::write(&file_path, content)?;

        let block_size = Generator::calculate_block_size(content.len() as u64);
        let generator = Generator::new(block_size, ChecksumAlgorithm::Md5);

        let checksums = generator.generate_checksums(&file_path)?;


        assert_eq!(checksums.len(), 1);
        assert_eq!(checksums[0].index, 0);
        assert_ne!(checksums[0].weak, 0);

        Ok(())
    }

    #[test]
    fn test_generate_checksums_multiple_blocks() -> Result<()> {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");


        let block_size = 10;
        let content = b"0123456789ABCDEFGHIJabcdefghij";

        let mut file = File::create(&file_path)?;
        file.write_all(content)?;
        file.flush()?;
        drop(file);

        let generator = Generator::new(block_size, ChecksumAlgorithm::Md5);
        let checksums = generator.generate_checksums(&file_path)?;


        assert_eq!(checksums.len(), 3);


        for (i, checksum) in checksums.iter().enumerate() {
            assert_eq!(checksum.index, i as u32);
        }


        assert_ne!(checksums[0].weak, checksums[1].weak);
        assert_ne!(checksums[1].weak, checksums[2].weak);

        Ok(())
    }

    #[test]
    fn test_generate_checksums_empty_file() -> Result<()> {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("empty.txt");


        fs::write(&file_path, b"")?;

        let generator = Generator::new(700, ChecksumAlgorithm::Md5);
        let checksums = generator.generate_checksums(&file_path)?;


        assert_eq!(checksums.len(), 0);

        Ok(())
    }

    #[test]
    fn test_generate_checksums_deterministic() -> Result<()> {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");

        let content = b"Deterministic test data";
        fs::write(&file_path, content)?;

        let generator = Generator::new(10, ChecksumAlgorithm::Md5);


        let checksums1 = generator.generate_checksums(&file_path)?;
        let checksums2 = generator.generate_checksums(&file_path)?;

        assert_eq!(checksums1.len(), checksums2.len());

        for (c1, c2) in checksums1.iter().zip(checksums2.iter()) {
            assert_eq!(c1.index, c2.index);
            assert_eq!(c1.weak, c2.weak);
            assert_eq!(c1.strong, c2.strong);
        }

        Ok(())
    }
}
