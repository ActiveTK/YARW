




use rayon::prelude::*;
use std::path::Path;
use crate::error::Result;
use crate::algorithm::checksum::{compute_strong_checksum, StrongChecksum};
use crate::algorithm::generator::BlockChecksum;
use crate::options::ChecksumAlgorithm;


pub struct ParallelChecksumEngine {
    algorithm: ChecksumAlgorithm,
    #[allow(dead_code)]
    num_threads: Option<usize>,
}

impl ParallelChecksumEngine {

    pub fn new(algorithm: ChecksumAlgorithm) -> Self {
        Self {
            algorithm,
            num_threads: None,
        }
    }


    #[allow(dead_code)]
    pub fn with_threads(mut self, num_threads: usize) -> Self {
        self.num_threads = Some(num_threads);
        self
    }




    pub fn compute_multiple(
        &self,
        files: &[&Path],
    ) -> Result<Vec<(usize, StrongChecksum)>> {

        let pool = if let Some(threads) = self.num_threads {
            rayon::ThreadPoolBuilder::new()
                .num_threads(threads)
                .build()
                .unwrap()
        } else {
            rayon::ThreadPoolBuilder::new()
                .build()
                .unwrap()
        };


        pool.install(|| {
            files
                .par_iter()
                .enumerate()
                .map(|(idx, file_path)| {
                    let data = std::fs::read(file_path)?;
                    let checksum = compute_strong_checksum(&data, &self.algorithm);
                    Ok((idx, checksum))
                })
                .collect()
        })
    }




    pub fn compute_block_checksums_parallel(
        &self,
        data: &[u8],
        block_size: usize,
    ) -> Vec<BlockChecksum> {
        use crate::algorithm::checksum::RollingChecksum;


        let blocks: Vec<_> = data
            .chunks(block_size)
            .enumerate()
            .collect();


        blocks
            .par_iter()
            .map(|(idx, block)| {

                let rolling = RollingChecksum::new(block);
                let weak = rolling.checksum();


                let strong = compute_strong_checksum(block, &self.algorithm);

                BlockChecksum {
                    index: *idx as u32,
                    weak,
                    strong,
                }
            })
            .collect()
    }
}

impl Default for ParallelChecksumEngine {
    fn default() -> Self {
        Self::new(ChecksumAlgorithm::Md5)
    }
}


pub fn compute_checksums_parallel(
    files: &[&Path],
    algorithm: ChecksumAlgorithm,
) -> Result<Vec<(usize, StrongChecksum)>> {
    let engine = ParallelChecksumEngine::new(algorithm);
    engine.compute_multiple(files)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs;

    #[test]
    fn test_parallel_checksum_multiple_files() -> Result<()> {
        let temp_dir = TempDir::new()?;


        let file1 = temp_dir.path().join("file1.txt");
        let file2 = temp_dir.path().join("file2.txt");
        let file3 = temp_dir.path().join("file3.txt");

        fs::write(&file1, b"content1")?;
        fs::write(&file2, b"content2")?;
        fs::write(&file3, b"content3")?;


        let files = vec![file1.as_path(), file2.as_path(), file3.as_path()];
        let engine = ParallelChecksumEngine::new(ChecksumAlgorithm::Md5);
        let results = engine.compute_multiple(&files)?;

        assert_eq!(results.len(), 3);


        for (idx, _checksum) in &results {
            assert!(*idx < 3);
        }

        Ok(())
    }

    #[test]
    fn test_parallel_block_checksums() {
        let data = b"This is a test data for block checksum calculation. It should be long enough.";
        let block_size = 16;

        let engine = ParallelChecksumEngine::new(ChecksumAlgorithm::Md5);
        let block_checksums = engine.compute_block_checksums_parallel(data, block_size);


        let expected_blocks = (data.len() + block_size - 1) / block_size;
        assert_eq!(block_checksums.len(), expected_blocks);


        for (i, block_checksum) in block_checksums.iter().enumerate() {
            assert_eq!(block_checksum.index, i as u32);
        }
    }

    #[test]
    fn test_parallel_checksum_deterministic() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let file = temp_dir.path().join("test.txt");
        fs::write(&file, b"deterministic test")?;

        let files = vec![file.as_path()];
        let engine = ParallelChecksumEngine::new(ChecksumAlgorithm::Md5);


        let result1 = engine.compute_multiple(&files)?;
        let result2 = engine.compute_multiple(&files)?;

        assert_eq!(result1.len(), result2.len());
        assert_eq!(result1[0].1.as_bytes(), result2[0].1.as_bytes());

        Ok(())
    }
}
