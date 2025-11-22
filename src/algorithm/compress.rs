use crate::options::CompressionAlgorithm;
use anyhow::Result;

pub struct Compressor {
    algorithm: CompressionAlgorithm,
}

impl Compressor {
    pub fn new(algorithm: CompressionAlgorithm) -> Self {
        Compressor { algorithm }
    }

    pub fn compress(&self, data: &[u8]) -> Result<Vec<u8>> {
        match self.algorithm {
            CompressionAlgorithm::Zstd => {
                let compressed = zstd::encode_all(data, 0)?;
                Ok(compressed)
            }
            CompressionAlgorithm::Lz4 => {
                let compressed = lz4_flex::compress_prepend_size(data);
                Ok(compressed)
            }
            CompressionAlgorithm::Zlib => {
                use flate2::write::ZlibEncoder;
                use flate2::Compression;
                use std::io::Write;

                let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
                encoder.write_all(data)?;
                let compressed = encoder.finish()?;
                Ok(compressed)
            }
        }
    }

    pub fn decompress(&self, data: &[u8]) -> Result<Vec<u8>> {
        match self.algorithm {
            CompressionAlgorithm::Zstd => {
                let decompressed = zstd::decode_all(data)?;
                Ok(decompressed)
            }
            CompressionAlgorithm::Lz4 => {
                let decompressed = lz4_flex::decompress_size_prepended(data)?;
                Ok(decompressed)
            }
            CompressionAlgorithm::Zlib => {
                use flate2::write::ZlibDecoder;
                use std::io::Write;

                let mut decoder = ZlibDecoder::new(Vec::new());
                decoder.write_all(data)?;
                let decompressed = decoder.finish()?;
                Ok(decompressed)
            }
        }
    }
}
