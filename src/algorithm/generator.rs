use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;
use crate::error::Result;
use crate::options::ChecksumAlgorithm;
use crate::algorithm::checksum::{RollingChecksum, StrongChecksum, compute_strong_checksum};

/// ブロックチェックサム（弱いチェックサムと強いチェックサムのペア）
#[derive(Debug, Clone)]
pub struct BlockChecksum {
    /// ブロックインデックス
    pub index: u32,
    /// 弱いチェックサム（Rolling checksum）
    pub weak: u32,
    /// 強いチェックサム
    pub strong: StrongChecksum,
}

/// ジェネレータ（受信側でファイルのブロックチェックサムを生成）
pub struct Generator {
    /// ブロックサイズ
    block_size: usize,
    /// Strong checksum アルゴリズム
    checksum_algorithm: ChecksumAlgorithm,
}

impl Generator {
    /// 新しいGeneratorを作成
    pub fn new(block_size: usize, checksum_algorithm: ChecksumAlgorithm) -> Self {
        Self {
            block_size,
            checksum_algorithm,
        }
    }

    /// ファイルサイズに基づいて最適なブロックサイズを計算
    /// 通常は sqrt(file_size) 程度が最適
    pub fn calculate_block_size(file_size: u64) -> usize {
        if file_size == 0 {
            return 700; // 最小ブロックサイズ
        }

        let size = (file_size as f64).sqrt() as usize;

        // ブロックサイズは 700B から 128KB の範囲
        size.max(700).min(128 * 1024)
    }

    /// 基準ファイルのブロックチェックサムリストを生成
    pub fn generate_checksums(&self, file: &Path) -> Result<Vec<BlockChecksum>> {
        let file = File::open(file)?;
        let mut reader = BufReader::new(file);
        let mut checksums = Vec::new();
        let mut buffer = vec![0u8; self.block_size];
        let mut index = 0u32;

        loop {
            let bytes_read = reader.read(&mut buffer)?;
            if bytes_read == 0 {
                break;
            }

            let block = &buffer[..bytes_read];

            // Rolling checksum（弱いチェックサム）
            let rolling = RollingChecksum::new(block);
            let weak = rolling.checksum();

            // Strong checksum（強いチェックサム）
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

    /// ブロックサイズを取得
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
        // 小さいファイル
        assert_eq!(Generator::calculate_block_size(0), 700);
        assert_eq!(Generator::calculate_block_size(1024), 700);

        // 中サイズファイル（1MB）
        let size_1mb = Generator::calculate_block_size(1024 * 1024);
        assert!(size_1mb >= 700 && size_1mb <= 128 * 1024);
        assert_eq!(size_1mb, 1024); // sqrt(1MB) = 1024

        // 大きいファイル（100MB）
        let size_100mb = Generator::calculate_block_size(100 * 1024 * 1024);
        assert!(size_100mb >= 700 && size_100mb <= 128 * 1024);

        // 非常に大きいファイル（10GB）
        let size_10gb = Generator::calculate_block_size(10u64 * 1024 * 1024 * 1024);
        assert!(size_10gb >= 700 && size_10gb <= 128 * 1024);
        // sqrt(10GB) = 約103621バイト ≈ 101KB

        // さらに大きいファイル（100GB） -> sqrt(100GB) = 約327KB → 最大値128KBに制限される
        let size_100gb = Generator::calculate_block_size(100u64 * 1024 * 1024 * 1024);
        assert_eq!(size_100gb, 128 * 1024);
    }

    #[test]
    fn test_generate_checksums_small_file() -> Result<()> {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");

        // 小さいファイルを作成
        let content = b"Hello, rsync!";
        fs::write(&file_path, content)?;

        let block_size = Generator::calculate_block_size(content.len() as u64);
        let generator = Generator::new(block_size, ChecksumAlgorithm::Md5);

        let checksums = generator.generate_checksums(&file_path)?;

        // 小さいファイルは1ブロック
        assert_eq!(checksums.len(), 1);
        assert_eq!(checksums[0].index, 0);
        assert_ne!(checksums[0].weak, 0);

        Ok(())
    }

    #[test]
    fn test_generate_checksums_multiple_blocks() -> Result<()> {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");

        // ブロックサイズ10バイトで30バイトのファイル = 3ブロック
        let block_size = 10;
        let content = b"0123456789ABCDEFGHIJabcdefghij"; // 30バイト

        let mut file = File::create(&file_path)?;
        file.write_all(content)?;
        file.flush()?;
        drop(file);

        let generator = Generator::new(block_size, ChecksumAlgorithm::Md5);
        let checksums = generator.generate_checksums(&file_path)?;

        // 3ブロック
        assert_eq!(checksums.len(), 3);

        // インデックスが連番であることを確認
        for (i, checksum) in checksums.iter().enumerate() {
            assert_eq!(checksum.index, i as u32);
        }

        // チェックサムが全て異なることを確認（異なるデータブロック）
        assert_ne!(checksums[0].weak, checksums[1].weak);
        assert_ne!(checksums[1].weak, checksums[2].weak);

        Ok(())
    }

    #[test]
    fn test_generate_checksums_empty_file() -> Result<()> {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("empty.txt");

        // 空のファイルを作成
        fs::write(&file_path, b"")?;

        let generator = Generator::new(700, ChecksumAlgorithm::Md5);
        let checksums = generator.generate_checksums(&file_path)?;

        // 空のファイルはチェックサムなし
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

        // 2回生成して同じ結果が得られることを確認
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
