use clap::{Parser, ArgAction};
use std::path::PathBuf;
use crate::options::{Options, CompressionAlgorithm, ChecksumAlgorithm};
use crate::error::{Result, RsyncError};
use crate::output::VerboseOutput;

#[derive(Parser, Debug)]
#[command(name = "rsync")]
#[command(author = "YARW: Yet Another Rsync for Windows")]
#[command(version)]
#[command(about = "A file synchronization tool for Windows", long_about = None)]
#[command(disable_help_flag = true)]
pub struct Cli {

    #[arg(long = "help", action = ArgAction::Help)]
    pub help: Option<bool>,

    #[arg(required = true)]
    pub source: Vec<String>,


    #[arg(required = true)]
    pub destination: String,



    #[arg(short = 'v', long = "verbose", action = ArgAction::Count)]
    pub verbose: u8,


    #[arg(short = 'q', long = "quiet")]
    pub quiet: bool,


    #[arg(short = 'c', long = "checksum")]
    pub checksum: bool,


    #[arg(short = 'a', long = "archive")]
    pub archive: bool,


    #[arg(short = 'r', long = "recursive")]
    pub recursive: bool,


    #[arg(short = 'R', long = "relative")]
    pub relative: bool,


    #[arg(short = 'u', long = "update")]
    pub update: bool,


    #[arg(short = 'l', long = "links")]
    pub links: bool,


    #[arg(short = 'L', long = "copy-links")]
    pub copy_links: bool,


    #[arg(short = 'H', long = "hard-links")]
    pub hard_links: bool,



    #[arg(short = 'p', long = "perms")]
    pub perms: bool,


    #[arg(short = 'g', long = "group")]
    pub group: bool,


    #[arg(short = 'o', long = "owner")]
    pub owner: bool,


    #[arg(short = 't', long = "times")]
    pub times: bool,


    #[arg(short = 'D')]
    pub devices_and_specials: bool,


    #[arg(long = "devices")]
    pub devices: bool,


    #[arg(long = "specials")]
    pub specials: bool,



    #[arg(short = 'z', long = "compress")]
    pub compress: bool,


    #[arg(long = "compress-choice")]
    pub compress_choice: Option<String>,


    #[arg(short = 'W', long = "whole-file")]
    pub whole_file: bool,


    #[arg(long = "inplace")]
    pub inplace: bool,


    #[arg(long = "partial")]
    pub partial: bool,


    #[arg(long = "partial-dir")]
    pub partial_dir: Option<PathBuf>,


    #[arg(short = 'b', long = "backup")]
    pub backup: bool,


    #[arg(long = "backup-dir")]
    pub backup_dir: Option<PathBuf>,


    #[arg(long = "suffix", default_value = "~")]
    pub suffix: String,


    #[arg(long = "bwlimit")]
    pub bwlimit: Option<u64>,



    #[arg(long = "delete")]
    pub delete: bool,


    #[arg(long = "delete-before")]
    pub delete_before: bool,


    #[arg(long = "delete-during")]
    pub delete_during: bool,


    #[arg(long = "delete-after")]
    pub delete_after: bool,


    #[arg(long = "delete-excluded")]
    pub delete_excluded: bool,


    #[arg(long = "remove-source-files")]
    pub remove_source_files: bool,



    #[arg(long = "exclude", action = ArgAction::Append)]
    pub exclude: Vec<String>,


    #[arg(long = "exclude-from")]
    pub exclude_from: Option<PathBuf>,


    #[arg(long = "include", action = ArgAction::Append)]
    pub include: Vec<String>,


    #[arg(long = "include-from")]
    pub include_from: Option<PathBuf>,


    #[arg(long = "files-from")]
    pub files_from: Option<PathBuf>,



    #[arg(long = "progress")]
    pub progress: bool,


    #[arg(short = 'i', long = "itemize-changes")]
    pub itemize_changes: bool,


    #[arg(long = "stats")]
    pub stats: bool,


    #[arg(short = 'h', long = "human-readable")]
    pub human_readable: bool,


    #[arg(long = "log-file")]
    pub log_file: Option<PathBuf>,



    #[arg(short = 'e', long = "rsh")]
    pub rsh: Option<String>,


    #[arg(long = "rsync-path")]
    pub rsync_path: Option<String>,



    #[arg(long = "daemon")]
    pub daemon: bool,


    #[arg(long = "address")]
    pub address: Option<String>,


    #[arg(long = "port")]
    pub port: Option<u16>,


    #[arg(long = "config")]
    pub config: Option<PathBuf>,


    #[arg(long = "password-file")]
    pub password_file: Option<PathBuf>,



    #[arg(short = 'n', long = "dry-run")]
    pub dry_run: bool,


    #[arg(long = "list-only")]
    pub list_only: bool,


    #[arg(long = "size-only")]
    pub size_only: bool,


    #[arg(long = "timeout")]
    pub timeout: Option<u64>,



    #[arg(long = "checksum-choice")]
    pub checksum_choice: Option<String>,
}

impl Cli {

    pub fn into_options(self) -> Result<Options> {
        let mut options = Options::default();


        options.verbose = self.verbose;
        options.quiet = self.quiet;
        options.checksum = self.checksum;
        options.archive = self.archive;
        options.recursive = self.recursive;
        options.relative = self.relative;
        options.update = self.update;
        options.links = self.links;
        options.copy_links = self.copy_links;
        options.hard_links = self.hard_links;


        options.compress = self.compress;
        if let Some(algo) = self.compress_choice {
            options.compress_choice = Some(parse_compression_algorithm(&algo)?);
        }
        options.whole_file = self.whole_file;
        options.inplace = self.inplace;
        options.partial = self.partial;
        options.partial_dir = self.partial_dir;
        options.bwlimit = self.bwlimit;


        options.backup = self.backup;
        options.backup_dir = self.backup_dir;
        options.suffix = self.suffix;


        options.delete = self.delete;
        options.delete_before = self.delete_before;
        options.delete_during = self.delete_during;
        options.delete_after = self.delete_after;
        options.delete_excluded = self.delete_excluded;
        options.remove_source_files = self.remove_source_files;


        options.exclude = self.exclude;
        options.include = self.include;
        options.exclude_from = self.exclude_from.into_iter().collect();
        options.include_from = self.include_from.into_iter().collect();
        options.files_from = self.files_from;


        options.progress = self.progress;
        options.itemize_changes = self.itemize_changes;
        options.stats = self.stats;
        options.human_readable = self.human_readable;
        options.log_file = self.log_file;


        options.rsh = self.rsh;
        options.rsync_path = self.rsync_path;


        options.daemon = self.daemon;
        options.address = self.address;
        if let Some(port) = self.port {
            options.port = Some(port);
        }
        options.config = self.config;
        options.password_file = self.password_file;


        options.dry_run = self.dry_run;
        options.list_only = self.list_only;
        options.size_only = self.size_only;
        options.timeout = self.timeout;


        if let Some(algo) = self.checksum_choice {
            options.checksum_choice = Some(parse_checksum_algorithm(&algo)?);
        }


        options.apply_archive_mode();

        let verbose = VerboseOutput::new(1, false);

        if self.perms {
            let warning = options.warn_unsupported_on_windows("perms");
            if !warning.is_empty() {
                verbose.print_warning(&warning);
            }
        }
        if self.group {
            let warning = options.warn_unsupported_on_windows("group");
            if !warning.is_empty() {
                verbose.print_warning(&warning);
            }
        }
        if self.owner {
            let warning = options.warn_unsupported_on_windows("owner");
            if !warning.is_empty() {
                verbose.print_warning(&warning);
            }
        }
        if self.times {
            let warning = options.warn_unsupported_on_windows("times");
            if !warning.is_empty() {
                verbose.print_warning(&warning);
            }
        }
        if self.devices_and_specials || self.devices || self.specials {
            let warning = options.warn_unsupported_on_windows("devices");
            if !warning.is_empty() {
                verbose.print_warning(&warning);
            }
        }

        Ok(options)
    }
}

fn parse_compression_algorithm(s: &str) -> Result<CompressionAlgorithm> {
    match s.to_lowercase().as_str() {
        "zstd" => Ok(CompressionAlgorithm::Zstd),
        "lz4" => Ok(CompressionAlgorithm::Lz4),
        "zlib" => Ok(CompressionAlgorithm::Zlib),
        _ => Err(RsyncError::InvalidOption(format!(
            "Invalid compression algorithm: {}. Valid options: zstd, lz4, zlib",
            s
        ))),
    }
}

fn parse_checksum_algorithm(s: &str) -> Result<ChecksumAlgorithm> {
    match s.to_lowercase().as_str() {
        "md4" => Ok(ChecksumAlgorithm::Md4),
        "md5" => Ok(ChecksumAlgorithm::Md5),
        "blake2" => Ok(ChecksumAlgorithm::Blake2),
        "xxh128" => Ok(ChecksumAlgorithm::Xxh128),
        _ => Err(RsyncError::InvalidOption(format!(
            "Invalid checksum algorithm: {}. Valid options: md4, md5, blake2, xxh128",
            s
        ))),
    }
}
