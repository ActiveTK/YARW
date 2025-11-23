use std::path::PathBuf;
use crate::output::VerboseOutput;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompressionAlgorithm {
    Zstd,
    Lz4,
    Zlib,
}

impl Default for CompressionAlgorithm {
    fn default() -> Self {
        CompressionAlgorithm::Zlib
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChecksumAlgorithm {
    Md4,
    Md5,
    Blake2,
    Xxh128,
}

impl Default for ChecksumAlgorithm {
    fn default() -> Self {
        ChecksumAlgorithm::Md5
    }
}


#[derive(Debug, Clone)]
pub struct Options {

    pub verbose: u8,
    pub quiet: bool,
    pub checksum: bool,
    pub archive: bool,
    pub recursive: bool,
    pub relative: bool,
    pub update: bool,
    pub links: bool,
    pub copy_links: bool,
    pub hard_links: bool,


    pub compress: bool,
    pub compress_choice: Option<CompressionAlgorithm>,
    pub whole_file: bool,
    pub inplace: bool,
    pub partial: bool,
    pub partial_dir: Option<PathBuf>,
    pub bwlimit: Option<u64>,


    pub backup: bool,
    pub backup_dir: Option<PathBuf>,
    pub suffix: String,


    pub delete: bool,
    pub delete_before: bool,
    pub delete_during: bool,
    pub delete_after: bool,
    pub delete_excluded: bool,
    pub remove_source_files: bool,


    pub exclude: Vec<String>,
    pub include: Vec<String>,
    pub exclude_from: Vec<PathBuf>,
    pub include_from: Vec<PathBuf>,
    pub files_from: Option<PathBuf>,


    pub progress: bool,
    pub itemize_changes: bool,
    pub stats: bool,
    pub human_readable: bool,
    pub log_file: Option<PathBuf>,


    pub rsh: Option<String>,
    pub rsync_path: Option<String>,


    pub daemon: bool,
    pub address: Option<String>,
    pub port: Option<u16>,
    pub config: Option<PathBuf>,
    pub password_file: Option<PathBuf>,


    pub dry_run: bool,
    pub list_only: bool,
    pub size_only: bool,
    pub timeout: Option<u64>,


    pub checksum_choice: Option<ChecksumAlgorithm>,
}

impl Default for Options {
    fn default() -> Self {
        Options {

            verbose: 0,
            quiet: false,
            checksum: false,
            archive: false,
            recursive: false,
            relative: false,
            update: false,
            links: false,
            copy_links: false,
            hard_links: false,


            compress: false,
            compress_choice: None,
            whole_file: false,
            inplace: false,
            partial: false,
            partial_dir: None,
            bwlimit: None,


            backup: false,
            backup_dir: None,
            suffix: "~".to_string(),


            delete: false,
            delete_before: false,
            delete_during: false,
            delete_after: false,
            delete_excluded: false,
            remove_source_files: false,


            exclude: Vec::new(),
            include: Vec::new(),
            exclude_from: Vec::new(),
            include_from: Vec::new(),
            files_from: None,


            progress: false,
            itemize_changes: false,
            stats: false,
            human_readable: false,
            log_file: None,


            rsh: None,
            rsync_path: None,


            daemon: false,
            address: None,
            port: Some(873),
            config: None,
            password_file: None,


            dry_run: false,
            list_only: false,
            size_only: false,
            timeout: None,


            checksum_choice: None,
        }
    }
}

impl Options {

    pub fn verbose_output(&self) -> VerboseOutput {
        VerboseOutput::new(self.verbose, self.quiet)
    }

    pub fn apply_archive_mode(&mut self) {
        if self.archive {
            self.recursive = true;
            self.links = true;
        }
    }

    pub fn warn_unsupported_on_windows(&self, opt: &str) -> String {
        format!("Warning: Option --{} (-{}) is not supported on Windows and will be ignored.", opt, &opt[..1])
    }
}
