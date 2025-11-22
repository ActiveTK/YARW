/// デルタ指示（送信側から受信側への指示）
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeltaInstruction {
    /// 基準ファイルのブロックをコピー
    /// index: 基準ファイルのブロックインデックス
    MatchedBlock { index: u32 },

    /// 新しいデータ（リテラルデータ）
    /// data: 送信する実際のバイトデータ
    LiteralData { data: Vec<u8> },
}

impl DeltaInstruction {
    /// MatchedBlock の作成
    pub fn matched_block(index: u32) -> Self {
        DeltaInstruction::MatchedBlock { index }
    }

    /// LiteralData の作成
    pub fn literal_data(data: Vec<u8>) -> Self {
        DeltaInstruction::LiteralData { data }
    }

    /// データサイズを取得（転送量の推定用）
    #[allow(dead_code)]
    pub fn size(&self) -> usize {
        match self {
            DeltaInstruction::MatchedBlock { .. } => {
                // ブロック参照は4バイト（インデックスのみ）
                4
            }
            DeltaInstruction::LiteralData { data } => {
                // リテラルデータはデータ長 + データ本体
                4 + data.len()
            }
        }
    }

    /// MatchedBlock かどうか
    #[allow(dead_code)]
    pub fn is_matched_block(&self) -> bool {
        matches!(self, DeltaInstruction::MatchedBlock { .. })
    }

    /// LiteralData かどうか
    #[allow(dead_code)]
    pub fn is_literal_data(&self) -> bool {
        matches!(self, DeltaInstruction::LiteralData { .. })
    }
}

/// デルタ指示リストの統計情報
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct DeltaStats {
    /// マッチしたブロック数
    pub matched_blocks: usize,
    /// リテラルデータのバイト数
    pub literal_bytes: usize,
    /// 総転送サイズ（推定）
    pub total_transfer_size: usize,
}

impl DeltaStats {
    /// デルタ指示リストから統計情報を計算
    #[allow(dead_code)]
    pub fn from_instructions(instructions: &[DeltaInstruction]) -> Self {
        let mut matched_blocks = 0;
        let mut literal_bytes = 0;
        let mut total_transfer_size = 0;

        for instruction in instructions {
            total_transfer_size += instruction.size();

            match instruction {
                DeltaInstruction::MatchedBlock { .. } => {
                    matched_blocks += 1;
                }
                DeltaInstruction::LiteralData { data } => {
                    literal_bytes += data.len();
                }
            }
        }

        Self {
            matched_blocks,
            literal_bytes,
            total_transfer_size,
        }
    }

    /// 圧縮率を計算（0.0 - 1.0）
    #[allow(dead_code)]
    pub fn compression_ratio(&self, original_size: usize) -> f64 {
        if original_size == 0 {
            return 0.0;
        }
        1.0 - (self.total_transfer_size as f64 / original_size as f64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_delta_instruction_size() {
        let matched = DeltaInstruction::matched_block(0);
        assert_eq!(matched.size(), 4);

        let literal = DeltaInstruction::literal_data(vec![1, 2, 3, 4, 5]);
        assert_eq!(literal.size(), 9); // 4 (length) + 5 (data)
    }

    #[test]
    fn test_delta_instruction_predicates() {
        let matched = DeltaInstruction::matched_block(0);
        assert!(matched.is_matched_block());
        assert!(!matched.is_literal_data());

        let literal = DeltaInstruction::literal_data(vec![1, 2, 3]);
        assert!(!literal.is_matched_block());
        assert!(literal.is_literal_data());
    }

    #[test]
    fn test_delta_stats_all_matched() {
        let instructions = vec![
            DeltaInstruction::matched_block(0),
            DeltaInstruction::matched_block(1),
            DeltaInstruction::matched_block(2),
        ];

        let stats = DeltaStats::from_instructions(&instructions);
        assert_eq!(stats.matched_blocks, 3);
        assert_eq!(stats.literal_bytes, 0);
        assert_eq!(stats.total_transfer_size, 12); // 3 * 4 bytes
    }

    #[test]
    fn test_delta_stats_all_literal() {
        let instructions = vec![
            DeltaInstruction::literal_data(vec![1, 2, 3]),
            DeltaInstruction::literal_data(vec![4, 5]),
        ];

        let stats = DeltaStats::from_instructions(&instructions);
        assert_eq!(stats.matched_blocks, 0);
        assert_eq!(stats.literal_bytes, 5);
        assert_eq!(stats.total_transfer_size, 13); // (4 + 3) + (4 + 2)
    }

    #[test]
    fn test_delta_stats_mixed() {
        let instructions = vec![
            DeltaInstruction::matched_block(0),
            DeltaInstruction::literal_data(vec![1, 2, 3, 4, 5]),
            DeltaInstruction::matched_block(1),
            DeltaInstruction::literal_data(vec![6, 7]),
        ];

        let stats = DeltaStats::from_instructions(&instructions);
        assert_eq!(stats.matched_blocks, 2);
        assert_eq!(stats.literal_bytes, 7);
        assert_eq!(stats.total_transfer_size, 23); // 4 + (4+5) + 4 + (4+2)
    }

    #[test]
    fn test_compression_ratio() {
        let instructions = vec![
            DeltaInstruction::matched_block(0),
            DeltaInstruction::matched_block(1),
        ];

        let stats = DeltaStats::from_instructions(&instructions);
        let original_size = 1000;

        // 転送サイズは8バイト、オリジナルは1000バイト
        // 圧縮率 = 1.0 - (8 / 1000) = 0.992
        let ratio = stats.compression_ratio(original_size);
        assert!((ratio - 0.992).abs() < 0.001);
    }

    #[test]
    fn test_compression_ratio_no_compression() {
        let data = vec![0u8; 1000];
        let instructions = vec![DeltaInstruction::literal_data(data)];

        let stats = DeltaStats::from_instructions(&instructions);
        let ratio = stats.compression_ratio(1000);

        // リテラルデータのみなので圧縮率は負（むしろ増加）
        assert!(ratio < 0.0);
    }
}
