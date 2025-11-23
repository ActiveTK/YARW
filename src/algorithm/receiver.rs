use std::fs::{File, OpenOptions};
use std::io::{BufReader, BufWriter, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use crate::error::{Result, RsyncError};
use crate::algorithm::delta::DeltaInstruction;
use crate::options::Options;
use crate::algorithm::compress::Compressor;
use crate::filesystem::buffer_optimizer::BufferOptimizer;
use tempfile::NamedTempFile;


pub struct Receiver {

    temp_dir: Option<PathBuf>,

    block_size: usize,

    compressor: Option<Compressor>,
}

impl Receiver {

    pub fn new(block_size: usize, options: &Options) -> Self {
        let compressor = if options.compress {
            Some(Compressor::new(options.compress_choice.unwrap_or_default()))
        } else {
            None
        };
        Self {
            temp_dir: None,
            block_size,
            compressor,
        }
    }


    #[allow(dead_code)]
    pub fn with_temp_dir(mut self, temp_dir: PathBuf) -> Self {
        self.temp_dir = Some(temp_dir);
        self
    }


    pub fn reconstruct_file(
        &self,
        base_file: Option<&Path>,
        delta: &[DeltaInstruction],
        output: &Path,
        options: &Options,
    ) -> Result<()> {
        if options.inplace {
            return self.reconstruct_file_inplace(base_file, delta, output);
        }

        let partial_path = if options.partial {
            if let Some(partial_dir) = &options.partial_dir {
                partial_dir.join(output.file_name().unwrap())
            } else {
                output.with_extension("partial")
            }
        } else {

            let temp_file = if let Some(temp_dir) = &self.temp_dir {
                NamedTempFile::new_in(temp_dir)?
            } else {
                NamedTempFile::new()?
            };
            temp_file.into_temp_path().to_path_buf()
        };


        let result = (|| -> Result<()> {
            let optimizer = BufferOptimizer::new();
            let writer_buffer_size = optimizer.optimal_buffer_for_file(&partial_path);
            let mut writer = BufWriter::with_capacity(writer_buffer_size, File::create(&partial_path)?);


            let mut base_reader = if let Some(base_path) = base_file {
                if base_path.exists() {
                    let reader_buffer_size = optimizer.optimal_buffer_for_file(base_path);
                    Some(BufReader::with_capacity(reader_buffer_size, File::open(base_path)?))
                } else {
                    None
                }
            } else {
                None
            };


            for instruction in delta {
                match instruction {
                    DeltaInstruction::MatchedBlock { index } => {
                        if let Some(ref mut reader) = base_reader {
                            let offset = (*index as u64) * (self.block_size as u64);
                            reader.seek(SeekFrom::Start(offset))?;
                            let mut block_buffer = vec![0u8; self.block_size];
                            let bytes_read = reader.read(&mut block_buffer)?;
                            writer.write_all(&block_buffer[..bytes_read])?;
                        } else {
                            return Err(RsyncError::Other(
                                "Matched block reference but no base file provided".to_string(),
                            ));
                        }
                    }
                    DeltaInstruction::LiteralData { data } => {
                        let data_to_write = if let Some(compressor) = &self.compressor {
                            compressor.decompress(data)?
                        } else {
                            data.clone()
                        };
                        writer.write_all(&data_to_write)?;
                    }
                }
            }
            writer.flush()?;
            Ok(())
        })();

        if result.is_ok() {

            std::fs::rename(&partial_path, output)?;
        } else {

            if !options.partial {
                let _ = std::fs::remove_file(&partial_path);
            }
        }

        result
    }

    fn reconstruct_file_inplace(
        &self,
        base_file: Option<&Path>,
        delta: &[DeltaInstruction],
        output: &Path,
    ) -> Result<()> {
        let optimizer = BufferOptimizer::new();
        let writer_buffer_size = optimizer.optimal_buffer_for_file(output);
        let mut writer = BufWriter::with_capacity(
            writer_buffer_size,
            OpenOptions::new().write(true).open(output)?
        );


        let mut base_reader = if let Some(base_path) = base_file {
            if base_path.exists() {
                let reader_buffer_size = optimizer.optimal_buffer_for_file(base_path);
                Some(BufReader::with_capacity(reader_buffer_size, File::open(base_path)?))
            } else {
                None
            }
        } else {
            None
        };

        for instruction in delta {
            match instruction {
                DeltaInstruction::MatchedBlock { index } => {
                    if let Some(ref mut reader) = base_reader {
                        let offset = (*index as u64) * (self.block_size as u64);
                        reader.seek(SeekFrom::Start(offset))?;
                        let mut block_buffer = vec![0u8; self.block_size];
                        let bytes_read = reader.read(&mut block_buffer)?;
                        writer.seek(SeekFrom::Current(0))?;
                        writer.write_all(&block_buffer[..bytes_read])?;
                    } else {
                        return Err(RsyncError::Other(
                            "Matched block reference but no base file provided".to_string(),
                        ));
                    }
                }
                DeltaInstruction::LiteralData { data } => {
                    let data_to_write = if let Some(compressor) = &self.compressor {
                        compressor.decompress(data)?
                    } else {
                        data.clone()
                    };
                    writer.seek(SeekFrom::Current(0))?;
                    writer.write_all(&data_to_write)?;
                }
            }
        }
        writer.flush()?;
        Ok(())
    }


    #[allow(dead_code)]
    pub fn verify_file(&self, file: &Path, expected_size: u64) -> Result<bool> {
        let metadata = std::fs::metadata(file)?;
        Ok(metadata.len() == expected_size)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::algorithm::generator::Generator;
    use crate::algorithm::sender::Sender;
    use crate::options::{ChecksumAlgorithm, Options};
    use tempfile::TempDir;
    use std::fs;

    #[test]
    fn test_reconstruct_identical_file() -> Result<()> {
        let options = Options::default();
        let temp_dir = TempDir::new().unwrap();
        let base_file = temp_dir.path().join("base.txt");
        let output_file = temp_dir.path().join("output.txt");

        let content = b"Hello, rsync! This is a test.";
        fs::write(&base_file, content)?;

        let block_size = 10;

        let generator = Generator::new(block_size, ChecksumAlgorithm::Md5);
        let checksums = generator.generate_checksums(&base_file)?;

        let mut sender = Sender::new(block_size, &options);
        let delta = sender.compute_delta(&base_file, &checksums, &options)?;

        let receiver = Receiver::new(block_size, &options);
        receiver.reconstruct_file(Some(&base_file), &delta, &output_file, &options)?;

        let reconstructed = fs::read(&output_file)?;
        assert_eq!(reconstructed, content);

        Ok(())
    }

    #[test]
    fn test_reconstruct_with_changes() -> Result<()> {
        let options = Options::default();
        let temp_dir = TempDir::new().unwrap();
        let base_file = temp_dir.path().join("base.txt");
        let source_file = temp_dir.path().join("source.txt");
        let output_file = temp_dir.path().join("output.txt");

        let base_content = b"AAAAAABBBBBBCCCCCC";
        fs::write(&base_file, base_content)?;

        let source_content = b"AAAAAADDDDDDCCCCCC";
        fs::write(&source_file, source_content)?;

        let block_size = 6;

        let generator = Generator::new(block_size, ChecksumAlgorithm::Md5);
        let checksums = generator.generate_checksums(&base_file)?;

        let mut sender = Sender::new(block_size, &options);
        let delta = sender.compute_delta(&source_file, &checksums, &options)?;

        let receiver = Receiver::new(block_size, &options);
        receiver.reconstruct_file(Some(&base_file), &delta, &output_file, &options)?;

        let reconstructed = fs::read(&output_file)?;
        assert_eq!(reconstructed, source_content);

        Ok(())
    }

    #[test]
    fn test_reconstruct_new_file() -> Result<()> {
        let options = Options::default();
        let temp_dir = TempDir::new().unwrap();
        let output_file = temp_dir.path().join("output.txt");

        let content = b"Brand new file content!";

        let delta = vec![DeltaInstruction::literal_data(content.to_vec())];

        let receiver = Receiver::new(10, &options);
        receiver.reconstruct_file(None, &delta, &output_file, &options)?;

        let reconstructed = fs::read(&output_file)?;
        assert_eq!(reconstructed, content);

        Ok(())
    }

    #[test]
    fn test_reconstruct_empty_delta() -> Result<()> {
        let options = Options::default();
        let temp_dir = TempDir::new().unwrap();
        let output_file = temp_dir.path().join("output.txt");

        let delta: Vec<DeltaInstruction> = vec![];

        let receiver = Receiver::new(10, &options);
        receiver.reconstruct_file(None, &delta, &output_file, &options)?;

        let reconstructed = fs::read(&output_file)?;
        assert!(reconstructed.is_empty());

        Ok(())
    }

    #[test]
    fn test_verify_file() -> Result<()> {
        let options = Options::default();
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");

        let content = b"Test content for verification";
        fs::write(&file_path, content)?;

        let receiver = Receiver::new(10, &options);

        assert!(receiver.verify_file(&file_path, content.len() as u64)?);
        assert!(!receiver.verify_file(&file_path, 999)?);

        Ok(())
    }

    #[test]
    fn test_end_to_end_rsync_algorithm() -> Result<()> {
        let options = Options::default();
        let temp_dir = TempDir::new().unwrap();
        let base_file = temp_dir.path().join("base.txt");
        let source_file = temp_dir.path().join("source.txt");
        let output_file = temp_dir.path().join("output.txt");

        let mut base_content = Vec::new();
        for i in 0..100 {
            base_content.extend_from_slice(format!("Line {} of base file\n", i).as_bytes());
        }
        fs::write(&base_file, &base_content)?;

        let mut source_content = base_content.clone();
        source_content.splice(50..60, b"MODIFIED".iter().cloned());
        fs::write(&source_file, &source_content)?;

        let block_size = 64;

        let generator = Generator::new(block_size, ChecksumAlgorithm::Md5);
        let checksums = generator.generate_checksums(&base_file)?;

        let mut sender = Sender::new(block_size, &options);
        let delta = sender.compute_delta(&source_file, &checksums, &options)?;

        let receiver = Receiver::new(block_size, &options);
        receiver.reconstruct_file(Some(&base_file), &delta, &output_file, &options)?;

        let reconstructed = fs::read(&output_file)?;
        assert_eq!(reconstructed, source_content);

        let delta_size: usize = delta.iter().map(|i| i.size()).sum();
        assert!(delta_size < source_content.len());

        Ok(())
    }
}
