




use rayon::prelude::*;
use crate::algorithm::checksum::compute_strong_checksum;
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

#[cfg(test)]
mod tests {
    use super::*;

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
}
