
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeltaInstruction {


    MatchedBlock { index: u32 },



    LiteralData { data: Vec<u8> },
}

impl DeltaInstruction {

    pub fn matched_block(index: u32) -> Self {
        DeltaInstruction::MatchedBlock { index }
    }


    pub fn literal_data(data: Vec<u8>) -> Self {
        DeltaInstruction::LiteralData { data }
    }


    #[allow(dead_code)]
    pub fn size(&self) -> usize {
        match self {
            DeltaInstruction::MatchedBlock { .. } => {

                4
            }
            DeltaInstruction::LiteralData { data } => {

                4 + data.len()
            }
        }
    }


    #[allow(dead_code)]
    pub fn is_matched_block(&self) -> bool {
        matches!(self, DeltaInstruction::MatchedBlock { .. })
    }


    #[allow(dead_code)]
    pub fn is_literal_data(&self) -> bool {
        matches!(self, DeltaInstruction::LiteralData { .. })
    }
}


#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct DeltaStats {

    pub matched_blocks: usize,

    pub literal_bytes: usize,

    pub total_transfer_size: usize,
}

impl DeltaStats {

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
        assert_eq!(literal.size(), 9);
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
        assert_eq!(stats.total_transfer_size, 12);
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
        assert_eq!(stats.total_transfer_size, 13);
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
        assert_eq!(stats.total_transfer_size, 23);
    }

    #[test]
    fn test_compression_ratio() {
        let instructions = vec![
            DeltaInstruction::matched_block(0),
            DeltaInstruction::matched_block(1),
        ];

        let stats = DeltaStats::from_instructions(&instructions);
        let original_size = 1000;



        let ratio = stats.compression_ratio(original_size);
        assert!((ratio - 0.992).abs() < 0.001);
    }

    #[test]
    fn test_compression_ratio_no_compression() {
        let data = vec![0u8; 1000];
        let instructions = vec![DeltaInstruction::literal_data(data)];

        let stats = DeltaStats::from_instructions(&instructions);
        let ratio = stats.compression_ratio(1000);


        assert!(ratio < 0.0);
    }
}
