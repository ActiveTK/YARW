use std::path::{Path, PathBuf};
use std::collections::HashMap;
use std::time::Instant;
use crate::error::Result;
use crate::options::{Options, ChecksumAlgorithm};
use crate::filesystem::{Scanner, FileInfo};
use crate::filesystem::file_info::human_readable_size;
use crate::algorithm::{Generator, Sender, Receiver, BandwidthLimiter, Compressor};
use crate::filter::FilterEngine;
use crate::output::{ProgressDisplay, ItemizeChange, VerboseOutput};


macro_rules! log_operation {
    ($($arg:tt)*) => {
        if crate::output::is_logging_enabled() {
            crate::output::log(&format!($($arg)*));
        }
    };
}


#[derive(Debug, Clone, Default)]
pub struct SyncStats {

    pub scanned_files: usize,

    pub transferred_files: usize,

    pub deleted_files: usize,

    pub transferred_bytes: u64,

    pub deleted_bytes: u64,

    pub unchanged_files: usize,

    pub execution_time_secs: f64,
}

impl SyncStats {

    pub fn display(&self, human_readable: bool, verbose: &VerboseOutput) {
        verbose.print_basic(&format!("\nNumber of files: {} (reg: {})",
            self.scanned_files,
            self.transferred_files + self.unchanged_files
        ));
        verbose.print_basic(&format!("Number of created files: {}", self.transferred_files));
        verbose.print_basic(&format!("Number of deleted files: {}", self.deleted_files));

        if human_readable {
            verbose.print_basic(&format!("Total file size: {}", human_readable_size(self.transferred_bytes)));
            verbose.print_basic(&format!("Deleted file size: {}", human_readable_size(self.deleted_bytes)));
        } else {
            verbose.print_basic(&format!("Total file size: {} bytes", self.transferred_bytes));
            verbose.print_basic(&format!("Deleted file size: {} bytes", self.deleted_bytes));
        }

        if self.execution_time_secs > 0.0 {
            verbose.print_transfer_rate(self.transferred_bytes, self.execution_time_secs);
            let speed = self.transferred_bytes as f64 / self.execution_time_secs;
            if human_readable {
                verbose.print_basic(&format!("Total transfer speed: {}/s", human_readable_size(speed as u64)));
            } else {
                verbose.print_basic(&format!("Total transfer speed: {:.2} bytes/s", speed));
            }
        }
    }
}


pub struct LocalTransport {
    options: Options,
}

impl LocalTransport {

    pub fn new(options: Options) -> Self {
        Self { options }
    }


    pub fn sync(&self, source: &Path, destination: &Path) -> Result<SyncStats> {
        let start_time = Instant::now();
        let mut stats = SyncStats::default();


        let source = dunce::canonicalize(source)?;
        let destination = if destination.exists() {
            dunce::canonicalize(destination)?
        } else {

            let parent = destination.parent().unwrap_or(destination);
            if parent.exists() {
                dunce::canonicalize(parent)?.join(destination.file_name().unwrap_or_default())
            } else {
                std::env::current_dir()?.join(destination)
            }
        };

        let verbose = self.options.verbose_output();
        verbose.print_basic(&format!("Syncing from {} to {}", source.display(), destination.display()));


        log_operation!("Starting sync: {} -> {}", source.display(), destination.display());


        if self.options.dry_run {
            verbose.print_basic("DRY RUN - no changes will be made");
            log_operation!("DRY RUN mode enabled");
        }


        let filter_engine = self.build_filter_engine()?;


        if !destination.exists() && !self.options.dry_run {
            std::fs::create_dir_all(&destination)?;
        }


        let scanner = Scanner::new()
            .recursive(self.options.recursive)
            .follow_symlinks(self.options.copy_links);

        let mut source_files = scanner.scan(&source)?;
        stats.scanned_files = source_files.len();

        verbose.print_verbose(&format!("Found {} files in source", source_files.len()));


        if let Some(ref files_from_path) = self.options.files_from {
            let allowed_files = crate::filesystem::read_files_from(files_from_path)?;

            verbose.print_verbose(&format!("Filtering {} files based on files-from list ({})",
                source_files.len(), files_from_path.display()));


            source_files.retain(|file_info| {
                let file_path = &file_info.path;

                allowed_files.iter().any(|allowed| {
                    file_path.ends_with(allowed) ||
                    file_path == allowed ||
                    allowed.ends_with(file_path.file_name().unwrap_or_default())
                })
            });

            verbose.print_verbose(&format!("After files-from filtering: {} files", source_files.len()));
        }


        let source_map = build_file_map(&source_files, &source, &filter_engine);

        verbose.print_verbose(&format!("Source map has {} entries", source_map.len()));


        if self.options.list_only {

            if !self.options.quiet {
                verbose.print_basic("File list:");
                for (rel_path, file_info) in &source_map {
                    if file_info.is_directory() {
                        verbose.print_basic(&format!("d         {} {}", file_info.size, rel_path.display()));
                    } else {
                        verbose.print_basic(&format!("f         {} {}", file_info.size, rel_path.display()));
                    }
                }
            }
            stats.scanned_files = source_map.len();
            return Ok(stats);
        }


        let dest_files = if destination.exists() {
            scanner.scan(&destination).unwrap_or_default()
        } else {
            Vec::new()
        };
        let dest_map = build_file_map(&dest_files, &destination, &filter_engine);


        let progress = if self.options.progress && !self.options.quiet {
            let total_bytes: u64 = source_map.values()
                .filter(|info| !info.is_directory())
                .map(|info| info.size)
                .sum();
            let file_count = source_map.values()
                .filter(|info| !info.is_directory())
                .count();
            Some(ProgressDisplay::new(total_bytes, file_count))
        } else {
            None
        };

        let mut transferred_bytes_so_far = 0u64;


        let mut bw_limiter = self.options.bwlimit.map(BandwidthLimiter::new);



        if self.options.delete && (self.options.delete_before || self.options.delete_during) {
            let deleted = self.delete_extra_files(&source_map, &dest_map, &destination)?;
            stats.deleted_files = deleted.len();
            for (path, size) in deleted {
                stats.deleted_bytes += size;
                if self.options.itemize_changes {
                    let change = ItemizeChange::delete_file(&path);
                    verbose.print_basic(&change.format());
                } else {
                    verbose.print_basic(&format!("deleting {}", path.display()));
                }
            }
        }


        for (rel_path, source_info) in &source_map {
            let dest_path = if self.options.relative {
                destination.join(source.strip_prefix(source.ancestors().nth(1).unwrap_or(&source)).unwrap_or(&source)).join(rel_path)
            } else {
                destination.join(rel_path)
            };

            if source_info.is_directory() {

                if !dest_path.exists() && !self.options.dry_run {
                    std::fs::create_dir_all(&dest_path)?;
                    verbose.print_basic(&format!("created directory {}", rel_path.display()));
                    if self.options.itemize_changes {
                        let change = ItemizeChange::new_directory(rel_path);
                        verbose.print_basic(&change.format());
                    }
                }
                continue;
            }

            let source_path = source.join(rel_path);


            if self.should_sync(&source_path, &dest_path, source_info, dest_map.get(rel_path))? {

                if self.options.itemize_changes {
                    let dest_info = dest_map.get(rel_path);
                    let size_diff = dest_info.map(|d| d.size != source_info.size).unwrap_or(true);
                    let time_diff = dest_info.map(|d| d.mtime != source_info.mtime).unwrap_or(true);

                    let change = if dest_info.is_none() {
                        ItemizeChange::new_file(rel_path)
                    } else {
                        ItemizeChange::update_file(rel_path, size_diff, time_diff)
                    };
                    verbose.print_basic(&change.format());
                } else {
                    verbose.print_basic(&format!("transferring {}", rel_path.display()));
                }


                if let Some(ref progress) = progress {
                    progress.update(transferred_bytes_so_far, &rel_path.to_string_lossy());
                }

                if !self.options.dry_run {
                    self.sync_file(&source_path, &dest_path, dest_map.get(rel_path))?;
                    log_operation!("Transferred: {} ({} bytes)", rel_path.display(), source_info.size);


                    if self.options.remove_source_files {
                        match std::fs::remove_file(&source_path) {
                            Ok(_) => {
                                verbose.print_verbose(&format!("removed source file {}", rel_path.display()));
                                log_operation!("Removed source: {}", rel_path.display());
                            }
                            Err(e) => {
                                verbose.print_warning(&format!("Failed to remove source file {}: {}", rel_path.display(), e));
                                log_operation!("Failed to remove source {}: {}", rel_path.display(), e);
                            }
                        }
                    }
                } else {
                    log_operation!("DRY RUN - Would transfer: {}", rel_path.display());
                    if self.options.remove_source_files {
                        log_operation!("DRY RUN - Would remove source: {}", rel_path.display());
                    }
                }

                stats.transferred_files += 1;
                stats.transferred_bytes += source_info.size;
                transferred_bytes_so_far += source_info.size;


                if let Some(ref mut limiter) = bw_limiter {
                    limiter.limit(source_info.size);
                }
            } else {
                stats.unchanged_files += 1;
                verbose.print_verbose(&format!("skipping {}", rel_path.display()));
            }
        }



        let should_delete_after = self.options.delete &&
            (self.options.delete_after ||
             (!self.options.delete_before && !self.options.delete_during));

        if should_delete_after {
            let deleted = self.delete_extra_files(&source_map, &dest_map, &destination)?;
            stats.deleted_files += deleted.len();
            for (path, size) in deleted {
                stats.deleted_bytes += size;
                if self.options.itemize_changes {
                    let change = ItemizeChange::delete_file(&path);
                    verbose.print_basic(&change.format());
                } else {
                    verbose.print_basic(&format!("deleting {}", path.display()));
                }
            }
        }


        if let Some(progress) = progress {
            progress.finish();
        }


        stats.execution_time_secs = start_time.elapsed().as_secs_f64();


        log_operation!(
            "Sync completed: {} files transferred, {} files deleted, {:.2} seconds",
            stats.transferred_files,
            stats.deleted_files,
            stats.execution_time_secs
        );

        Ok(stats)
    }


    fn build_filter_engine(&self) -> Result<FilterEngine> {
        let mut engine = FilterEngine::new();


        for pattern in &self.options.exclude {
            engine.add_exclude(pattern)?;
        }


        for pattern in &self.options.include {
            engine.add_include(pattern)?;
        }


        for file_path in &self.options.exclude_from {
            engine.add_exclude_from(file_path)?;
        }


        for file_path in &self.options.include_from {
            engine.add_include_from(file_path)?;
        }

        let verbose = self.options.verbose_output();
        verbose.print_verbose(&format!("Loaded {} filter pattern(s)", engine.pattern_count()));

        Ok(engine)
    }


    fn should_sync(
        &self,
        source_path: &Path,
        dest_path: &Path,
        source_info: &FileInfo,
        dest_info: Option<&FileInfo>,
    ) -> Result<bool> {

        let Some(dest_info) = dest_info else {
            return Ok(true);
        };


        if self.options.update {
            if dest_info.mtime > source_info.mtime {
                return Ok(false);
            }
        }


        if self.options.size_only {
            return Ok(source_info.size != dest_info.size);
        }


        if self.options.checksum {
            let source_checksum = self.compute_file_checksum(source_path)?;
            let dest_checksum = self.compute_file_checksum(dest_path)?;
            return Ok(source_checksum != dest_checksum);
        }


        Ok(source_info.size != dest_info.size || source_info.mtime != dest_info.mtime)
    }


    fn sync_file(
        &self,
        source: &Path,
        destination: &Path,
        base_info: Option<&FileInfo>,
    ) -> Result<()> {

        if let Some(parent) = destination.parent() {
            std::fs::create_dir_all(parent)?;
        }


        if self.options.backup && destination.exists() {
            self.create_backup(destination)?;
        }


        if self.options.whole_file || base_info.is_none() {

            if self.options.compress {
                self.copy_with_compression(source, destination)?;
            } else {
                std::fs::copy(source, destination)?;
            }
            return Ok(());
        }


        let block_size = Generator::calculate_block_size(
            std::fs::metadata(source)?.len()
        );

        let checksum_algorithm = self.options.checksum_choice
            .clone()
            .unwrap_or(ChecksumAlgorithm::Md5);


        let generator = Generator::new(block_size, checksum_algorithm);
        let checksums = generator.generate_checksums(destination)?;


        let mut sender = Sender::new(block_size, &self.options);
        let delta = sender.compute_delta(source, &checksums, &self.options)?;


        let receiver = Receiver::new(block_size, &self.options);
        receiver.reconstruct_file(Some(destination), &delta, destination, &self.options)?;

        Ok(())
    }



    fn copy_with_compression(&self, source: &Path, destination: &Path) -> Result<()> {
        use std::io::Write;


        let algorithm = self.options.compress_choice
            .unwrap_or(crate::options::CompressionAlgorithm::Zlib);

        let compressor = Compressor::new(algorithm);


        let data = std::fs::read(source)?;
        let original_size = data.len();


        let compressed = compressor.compress(&data)
            .map_err(|e| crate::error::RsyncError::Io(
                std::io::Error::new(std::io::ErrorKind::Other, e.to_string())
            ))?;
        let compressed_size = compressed.len();


        let decompressed = compressor.decompress(&compressed)
            .map_err(|e| crate::error::RsyncError::Io(
                std::io::Error::new(std::io::ErrorKind::Other, e.to_string())
            ))?;


        let mut file = std::fs::File::create(destination)?;
        file.write_all(&decompressed)?;


        let verbose = self.options.verbose_output();
        let ratio = if original_size > 0 {
            (compressed_size as f64 / original_size as f64) * 100.0
        } else {
            100.0
        };
        verbose.print_verbose(&format!(
            "  Compressed: {} -> {} bytes ({:.1}%)",
            original_size, compressed_size, ratio
        ));

        log_operation!(
            "Compressed transfer: {} bytes -> {} bytes ({:.1}% ratio)",
            original_size,
            compressed_size,
            if original_size > 0 { (compressed_size as f64 / original_size as f64) * 100.0 } else { 100.0 }
        );

        Ok(())
    }


    fn delete_extra_files(
        &self,
        source_map: &HashMap<PathBuf, FileInfo>,
        dest_map: &HashMap<PathBuf, FileInfo>,
        destination: &Path,
    ) -> Result<Vec<(PathBuf, u64)>> {
        let mut deleted = Vec::new();

        for (rel_path, dest_info) in dest_map {

            if !source_map.contains_key(rel_path) {
                let full_path = destination.join(rel_path);
                let size = dest_info.size;

                if !self.options.dry_run {
                    if dest_info.is_directory() {
                        std::fs::remove_dir_all(&full_path)?;
                        log_operation!("Deleted directory: {}", rel_path.display());
                    } else {
                        std::fs::remove_file(&full_path)?;
                        log_operation!("Deleted file: {} ({} bytes)", rel_path.display(), size);
                    }
                } else {
                    log_operation!("DRY RUN - Would delete: {}", rel_path.display());
                }

                deleted.push((rel_path.clone(), size));
            }
        }

        Ok(deleted)
    }


    fn compute_file_checksum(&self, path: &Path) -> Result<Vec<u8>> {
        use crate::algorithm::checksum::compute_strong_checksum;

        let data = std::fs::read(path)?;
        let algo = self.options.checksum_choice.unwrap_or(ChecksumAlgorithm::Md5);
        let checksum = compute_strong_checksum(&data, &algo);

        Ok(checksum.as_bytes().to_vec())
    }


    fn create_backup(&self, file: &Path) -> Result<()> {
        let verbose = self.options.verbose_output();

        if let Some(ref backup_dir) = self.options.backup_dir {


            let backup_path = backup_dir.join(file.file_name().unwrap_or_default());


            if let Some(parent) = backup_path.parent() {
                std::fs::create_dir_all(parent)?;
            }

            std::fs::copy(file, &backup_path)?;

            verbose.print_verbose(&format!("backed up {} to {}", file.display(), backup_path.display()));
        } else {

            let backup_path = file.with_extension(
                format!("{}{}",
                    file.extension().and_then(|e| e.to_str()).unwrap_or(""),
                    self.options.suffix
                )
            );


            let backup_path = if file.extension().is_none() {
                PathBuf::from(format!("{}{}", file.display(), self.options.suffix))
            } else {
                backup_path
            };

            std::fs::copy(file, &backup_path)?;

            verbose.print_verbose(&format!("backed up {} to {}", file.display(), backup_path.display()));
        }

        Ok(())
    }
}


fn build_file_map(files: &[FileInfo], base: &Path, filter: &FilterEngine) -> HashMap<PathBuf, FileInfo> {
    let mut map = HashMap::new();

    for file_info in files {

        let rel_path = match file_info.relative_path(base) {
            Some(path) => path,
            None => continue,
        };


        if !filter.should_include(&rel_path) {
            continue;
        }

        map.insert(rel_path, file_info.clone());
    }

    map
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs;

    fn create_test_options() -> Options {
        let mut options = Options::default();
        options.recursive = true;
        options
    }

    #[test]
    fn test_sync_new_directory() -> Result<()> {
        let temp_dir = TempDir::new().unwrap();
        let source = temp_dir.path().join("source");
        let dest = temp_dir.path().join("dest");


        fs::create_dir(&source)?;
        fs::write(source.join("file1.txt"), b"content1")?;
        fs::write(source.join("file2.txt"), b"content2")?;

        let transport = LocalTransport::new(create_test_options());
        let stats = transport.sync(&source, &dest)?;


        assert!(dest.join("file1.txt").exists());
        assert!(dest.join("file2.txt").exists());
        assert_eq!(fs::read(dest.join("file1.txt"))?, b"content1");
        assert_eq!(stats.transferred_files, 2);

        Ok(())
    }

    #[test]
    fn test_sync_with_delete() -> Result<()> {
        let temp_dir = TempDir::new().unwrap();
        let source = temp_dir.path().join("source");
        let dest = temp_dir.path().join("dest");


        fs::create_dir(&source)?;
        fs::create_dir(&dest)?;
        fs::write(source.join("file1.txt"), b"content1")?;
        fs::write(dest.join("file2.txt"), b"extra")?;

        let mut options = create_test_options();
        options.delete = true;

        let transport = LocalTransport::new(options);
        let stats = transport.sync(&source, &dest)?;


        assert!(dest.join("file1.txt").exists());
        assert!(!dest.join("file2.txt").exists());
        assert_eq!(stats.deleted_files, 1);

        Ok(())
    }

    #[test]
    fn test_sync_unchanged_files() -> Result<()> {
        let temp_dir = TempDir::new().unwrap();
        let source = temp_dir.path().join("source");
        let dest = temp_dir.path().join("dest");


        fs::create_dir(&source)?;
        fs::create_dir(&dest)?;
        fs::write(source.join("file.txt"), b"same content")?;


        std::thread::sleep(std::time::Duration::from_millis(10));
        fs::write(dest.join("file.txt"), b"same content")?;


        let mut options = create_test_options();
        options.size_only = true;

        let transport = LocalTransport::new(options);
        let stats = transport.sync(&source, &dest)?;


        assert_eq!(stats.unchanged_files, 1);
        assert_eq!(stats.transferred_files, 0);

        Ok(())
    }

    #[test]
    fn test_sync_dry_run() -> Result<()> {
        let temp_dir = TempDir::new().unwrap();
        let source = temp_dir.path().join("source");
        let dest = temp_dir.path().join("dest");

        fs::create_dir(&source)?;
        fs::write(source.join("file.txt"), b"content")?;

        let mut options = create_test_options();
        options.dry_run = true;

        let transport = LocalTransport::new(options);
        let _stats = transport.sync(&source, &dest)?;


        assert!(!dest.exists());

        Ok(())
    }
}
