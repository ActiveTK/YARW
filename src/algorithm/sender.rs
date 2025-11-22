use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;
use crate::error::Result;
use crate::algorithm::checksum::RollingChecksum;
use crate::algorithm::generator::BlockChecksum;
use crate::algorithm::delta::DeltaInstruction;
use crate::options::Options;
use crate::algorithm::compress::Compressor;
use crate::algorithm::bwlimit::BandwidthLimiter;

/// 送信者（ソースファイルをスキャンして差分を計算）
pub struct Sender {
    /// ブロックサイズ
    block_size: usize,
    /// 圧縮アルゴリズム
    compressor: Option<Compressor>,
    /// 帯域幅リミッター
    bandwidth_limiter: Option<BandwidthLimiter>,
}

impl Sender {
    /// 新しいSenderを作成
    pub fn new(block_size: usize, options: &Options) -> Self {
        let compressor = if options.compress {
            Some(Compressor::new(options.compress_choice.unwrap_or_default()))
        } else {
            None
        };
        let bandwidth_limiter = if let Some(bwlimit) = options.bwlimit {
            Some(BandwidthLimiter::new(bwlimit * 1024))
        } else {
            None
        };
        Self { block_size, compressor, bandwidth_limiter }
    }

    /// ブロックチェックサムからハッシュテーブルを構築
    /// 弱いチェックサムをキーにして、該当するブロックのリストを値とする
    pub fn build_hash_table<'a>(
        checksums: &'a [BlockChecksum],
    ) -> HashMap<u32, Vec<&'a BlockChecksum>> {
        let mut hash_table: HashMap<u32, Vec<&'a BlockChecksum>> = HashMap::new();

        for checksum in checksums {
            hash_table
                .entry(checksum.weak)
                .or_insert_with(Vec::new)
                .push(checksum);
        }

        hash_table
    }

    /// ソースファイルをスキャンして差分を計算
    pub fn compute_delta(
        &mut self,
        source: &Path,
        checksums: &[BlockChecksum],
        options: &Options,
    ) -> Result<Vec<DeltaInstruction>> {
        let hash_table = Self::build_hash_table(checksums);
        let file = File::open(source)?;
        let mut reader = BufReader::new(file);
        let mut buffer = Vec::new();
        reader.read_to_end(&mut buffer)?;

        if buffer.is_empty() {
            return Ok(Vec::new());
        }

        let mut instructions = Vec::new();
        let mut pos = 0;
        let mut literal_buffer = Vec::new();
        let mut rolling_checksum: Option<RollingChecksum> = None;

        // フルブロックが取れる間、ローリングスキャンを実行
        while pos + self.block_size <= buffer.len() {
            let weak = if let Some(ref mut rolling) = rolling_checksum {
                // roll() を使って効率的に更新
                let old_byte = buffer[pos - 1];
                let new_byte = buffer[pos + self.block_size - 1];
                rolling.roll(old_byte, new_byte);
                rolling.checksum()
            } else {
                // 最初のブロックまたはマッチ後の再初期化
                let block = &buffer[pos..pos + self.block_size];
                let rolling = RollingChecksum::new(block);
                let weak_checksum = rolling.checksum();
                rolling_checksum = Some(rolling);
                weak_checksum
            };

            let mut matched = false;
            if let Some(candidates) = hash_table.get(&weak) {
                let block = &buffer[pos..pos + self.block_size];
                let strong = crate::algorithm::checksum::compute_strong_checksum(
                    block,
                    &options.checksum_choice.unwrap_or_default(),
                );

                if let Some(matched_block) = candidates.iter().find(|c| c.strong == strong) {
                    if !literal_buffer.is_empty() {
                        let data_to_send = self.compress_and_limit(&mut literal_buffer)?;
                        instructions.push(DeltaInstruction::literal_data(data_to_send));
                        literal_buffer.clear();
                    }

                    instructions.push(DeltaInstruction::matched_block(matched_block.index));
                    pos += self.block_size;
                    rolling_checksum = None; // 次のブロックで再初期化
                    matched = true;
                }
            }

            if !matched {
                literal_buffer.push(buffer[pos]);
                pos += 1;
            }
        }

        // ファイル末尾の残りの部分を処理
        if pos < buffer.len() {
            let final_block = &buffer[pos..];
            let weak = RollingChecksum::new(final_block).checksum();
            let mut final_match = false;

            if let Some(candidates) = hash_table.get(&weak) {
                let strong = crate::algorithm::checksum::compute_strong_checksum(
                    final_block,
                    &options.checksum_choice.unwrap_or_default(),
                );
                if let Some(matched_block) = candidates.iter().find(|c| c.strong == strong) {
                    if !literal_buffer.is_empty() {
                        let data_to_send = self.compress_and_limit(&mut literal_buffer)?;
                        instructions.push(DeltaInstruction::literal_data(data_to_send));
                        literal_buffer.clear();
                    }
                    instructions.push(DeltaInstruction::matched_block(matched_block.index));
                    final_match = true;
                }
            }

            if !final_match {
                literal_buffer.extend_from_slice(final_block);
            }
        }


        if !literal_buffer.is_empty() {
            let data_to_send = self.compress_and_limit(&mut literal_buffer)?;
            instructions.push(DeltaInstruction::literal_data(data_to_send));
        }

        Ok(instructions)
    }

    fn compress_and_limit(&mut self, data: &mut Vec<u8>) -> Result<Vec<u8>> {
        let compressed_data = if let Some(compressor) = &self.compressor {
            compressor.compress(data)?
        } else {
            data.clone()
        };

        if let Some(limiter) = &mut self.bandwidth_limiter {
            limiter.limit(compressed_data.len() as u64);
        }

        Ok(compressed_data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::algorithm::checksum::StrongChecksum;
    use crate::algorithm::generator::Generator;
    use crate::options::{ChecksumAlgorithm, Options};
    use tempfile::TempDir;
    use std::fs;

    #[test]
    fn test_build_hash_table() {
        let checksums = vec![
            BlockChecksum {
                index: 0,
                weak: 100,
                strong: StrongChecksum::Md5([0; 16]),
            },
            BlockChecksum {
                index: 1,
                weak: 200,
                strong: StrongChecksum::Md5([1; 16]),
            },
            BlockChecksum {
                index: 2,
                weak: 100, // 同じ弱いチェックサム
                strong: StrongChecksum::Md5([2; 16]),
            },
        ];

        let hash_table = Sender::build_hash_table(&checksums);

        assert_eq!(hash_table.len(), 2);
        assert_eq!(hash_table.get(&100).unwrap().len(), 2);
        assert_eq!(hash_table.get(&200).unwrap().len(), 1);
    }

    #[test]
    fn test_compute_delta_identical_files() -> Result<()> {
        let options = Options::default();
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");

        let content = b"Hello, this is a test file for rsync algorithm!";
        fs::write(&file_path, content)?;

        let block_size = 10;
        let generator = Generator::new(block_size, ChecksumAlgorithm::Md5);
        let checksums = generator.generate_checksums(&file_path)?;

        let mut sender = Sender::new(block_size, &options);
        let delta = sender.compute_delta(&file_path, &checksums, &options)?;

        // 完全に一致するファイルは全てMatchedBlockのはず
        for instruction in &delta {
            assert!(instruction.is_matched_block(), "Instruction was not a matched block: {:?}", instruction);
        }

        Ok(())
    }

    #[test]
    fn test_compute_delta_completely_different() -> Result<()> {
        let options = Options::default();
        let temp_dir = TempDir::new().unwrap();
        let base_file = temp_dir.path().join("base.txt");
        let source_file = temp_dir.path().join("source.txt");

        fs::write(&base_file, b"AAAAAAAAAA")?;
        fs::write(&source_file, b"BBBBBBBBBB")?;

        let block_size = 10;
        let generator = Generator::new(block_size, ChecksumAlgorithm::Md5);
        let checksums = generator.generate_checksums(&base_file)?;

        let mut sender = Sender::new(block_size, &options);
        let delta = sender.compute_delta(&source_file, &checksums, &options)?;

        // 完全に異なるファイルは主にLiteralDataのはず
        let literal_count = delta.iter().filter(|i| i.is_literal_data()).count();
        assert!(literal_count > 0);

        Ok(())
    }

    #[test]
    fn test_compute_delta_partial_match() -> Result<()> {
        let options = Options::default();
        let temp_dir = TempDir::new().unwrap();
        let base_file = temp_dir.path().join("base.txt");
        let source_file = temp_dir.path().join("source.txt");

        // ベースファイル: "AAAAAABBBBBBCCCCCC"
        let base_content = b"AAAAAABBBBBBCCCCCC";
        fs::write(&base_file, base_content)?;

        // ソースファイル: "AAAAAADDDDDDCCCCCC" (真ん中が変更)
        let source_content = b"AAAAAADDDDDDCCCCCC";
        fs::write(&source_file, source_content)?;

        let block_size = 6;
        let generator = Generator::new(block_size, ChecksumAlgorithm::Md5);
        let checksums = generator.generate_checksums(&base_file)?;

        let mut sender = Sender::new(block_size, &options);
        let delta = sender.compute_delta(&source_file, &checksums, &options)?;

        // マッチしたブロックとリテラルデータの両方が含まれるはず
        let matched_count = delta.iter().filter(|i| i.is_matched_block()).count();
        let literal_count = delta.iter().filter(|i| i.is_literal_data()).count();

        assert!(matched_count > 0, "Should have matched blocks");
        assert!(literal_count > 0, "Should have literal data");

        Ok(())
    }

    #[test]
    fn test_compute_delta_empty_file() -> Result<()> {
        let options = Options::default();
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("empty.txt");

        fs::write(&file_path, b"")?;

        let mut sender = Sender::new(10, &options);
        let delta = sender.compute_delta(&file_path, &[], &options)?;

        assert_eq!(delta.len(), 0);

        Ok(())
    }
}
