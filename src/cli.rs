use clap::{Parser, ArgAction};
use std::path::PathBuf;
use crate::options::{Options, CompressionAlgorithm, ChecksumAlgorithm};
use crate::error::{Result, RsyncError};

#[derive(Parser, Debug)]
#[command(name = "rsync")]
#[command(author = "YARW: Yet Another Rsync for Windows")]
#[command(version = "0.1.0")]
#[command(about = "A file synchronization tool for Windows", long_about = None)]
#[command(disable_help_flag = true)]
pub struct Cli {
    /// Print help information (use --help)
    #[arg(long = "help", action = ArgAction::Help)]
    pub help: Option<bool>,
    /// Source path(s)
    #[arg(required = true)]
    pub source: Vec<String>,

    /// Destination path
    #[arg(required = true)]
    pub destination: String,

    // 基本オプション
    /// Increase verbosity (-v, -vv, -vvv)
    #[arg(short = 'v', long = "verbose", action = ArgAction::Count)]
    pub verbose: u8,

    /// Suppress non-error messages
    #[arg(short = 'q', long = "quiet")]
    pub quiet: bool,

    /// Skip based on checksum, not mod-time & size
    #[arg(short = 'c', long = "checksum")]
    pub checksum: bool,

    /// Archive mode; equals -rl on Windows (not -rlptgoD)
    #[arg(short = 'a', long = "archive")]
    pub archive: bool,

    /// Recurse into directories
    #[arg(short = 'r', long = "recursive")]
    pub recursive: bool,

    /// Use relative path names
    #[arg(short = 'R', long = "relative")]
    pub relative: bool,

    /// Skip files that are newer on the receiver
    #[arg(short = 'u', long = "update")]
    pub update: bool,

    /// Copy symlinks as symlinks
    #[arg(short = 'l', long = "links")]
    pub links: bool,

    /// Transform symlink into referent file/dir
    #[arg(short = 'L', long = "copy-links")]
    pub copy_links: bool,

    /// Preserve hard links
    #[arg(short = 'H', long = "hard-links")]
    pub hard_links: bool,

    // Windows非対応オプション（警告のみ）
    /// Preserve permissions (NOT SUPPORTED ON WINDOWS - warning only)
    #[arg(short = 'p', long = "perms")]
    pub perms: bool,

    /// Preserve group (NOT SUPPORTED ON WINDOWS - warning only)
    #[arg(short = 'g', long = "group")]
    pub group: bool,

    /// Preserve owner (NOT SUPPORTED ON WINDOWS - warning only)
    #[arg(short = 'o', long = "owner")]
    pub owner: bool,

    /// Preserve modification times (NOT SUPPORTED ON WINDOWS - warning only)
    #[arg(short = 't', long = "times")]
    pub times: bool,

    /// Preserve device files (NOT SUPPORTED ON WINDOWS - warning only)
    #[arg(short = 'D')]
    pub devices_and_specials: bool,

    /// Preserve device files (NOT SUPPORTED ON WINDOWS - warning only)
    #[arg(long = "devices")]
    pub devices: bool,

    /// Preserve special files (NOT SUPPORTED ON WINDOWS - warning only)
    #[arg(long = "specials")]
    pub specials: bool,

    // 転送オプション
    /// Compress file data during the transfer
    #[arg(short = 'z', long = "compress")]
    pub compress: bool,

    /// Choose compression algorithm (zstd, lz4, zlib)
    #[arg(long = "compress-choice")]
    pub compress_choice: Option<String>,

    /// Copy files whole (without delta-xfer algorithm)
    #[arg(short = 'W', long = "whole-file")]
    pub whole_file: bool,

    /// Update destination files in-place
    #[arg(long = "inplace")]
    pub inplace: bool,

    /// Keep partially transferred files
    #[arg(long = "partial")]
    pub partial: bool,

    /// Put partial files into DIR
    #[arg(long = "partial-dir")]
    pub partial_dir: Option<PathBuf>,

    /// Make backups (see --suffix & --backup-dir)
    #[arg(short = 'b', long = "backup")]
    pub backup: bool,

    /// Make backups into hierarchy based in DIR
    #[arg(long = "backup-dir")]
    pub backup_dir: Option<PathBuf>,

    /// Set backup suffix (default ~ without --backup-dir)
    #[arg(long = "suffix", default_value = "~")]
    pub suffix: String,

    /// Limit socket I/O bandwidth (KB/s)
    #[arg(long = "bwlimit")]
    pub bwlimit: Option<u64>,

    // 削除オプション
    /// Delete extraneous files from dest dirs
    #[arg(long = "delete")]
    pub delete: bool,

    /// Receiver deletes before transfer (default)
    #[arg(long = "delete-before")]
    pub delete_before: bool,

    /// Receiver deletes during transfer
    #[arg(long = "delete-during")]
    pub delete_during: bool,

    /// Receiver deletes after transfer
    #[arg(long = "delete-after")]
    pub delete_after: bool,

    /// Also delete excluded files from dest dirs
    #[arg(long = "delete-excluded")]
    pub delete_excluded: bool,

    /// Sender removes synchronized files (non-dir)
    #[arg(long = "remove-source-files")]
    pub remove_source_files: bool,

    // フィルタリングオプション
    /// Exclude files matching PATTERN
    #[arg(long = "exclude", action = ArgAction::Append)]
    pub exclude: Vec<String>,

    /// Read exclude patterns from FILE
    #[arg(long = "exclude-from")]
    pub exclude_from: Option<PathBuf>,

    /// Don't exclude files matching PATTERN
    #[arg(long = "include", action = ArgAction::Append)]
    pub include: Vec<String>,

    /// Read include patterns from FILE
    #[arg(long = "include-from")]
    pub include_from: Option<PathBuf>,

    /// Read list of source files from FILE
    #[arg(long = "files-from")]
    pub files_from: Option<PathBuf>,

    // 出力・表示オプション
    /// Show progress during transfer
    #[arg(long = "progress")]
    pub progress: bool,

    /// Output a change-summary for all updates
    #[arg(short = 'i', long = "itemize-changes")]
    pub itemize_changes: bool,

    /// Give some file-transfer stats
    #[arg(long = "stats")]
    pub stats: bool,

    /// Output numbers in a human-readable format
    #[arg(short = 'h', long = "human-readable")]
    pub human_readable: bool,

    /// Log what we're doing to the specified FILE
    #[arg(long = "log-file")]
    pub log_file: Option<PathBuf>,

    // リモート転送オプション
    /// Specify the remote shell to use
    #[arg(short = 'e', long = "rsh")]
    pub rsh: Option<String>,

    /// Specify path to rsync on the remote machine
    #[arg(long = "rsync-path")]
    pub rsync_path: Option<String>,

    // デーモンモードオプション
    /// Run as an rsync daemon
    #[arg(long = "daemon")]
    pub daemon: bool,

    /// Bind to the specified address
    #[arg(long = "address")]
    pub address: Option<String>,

    /// Specify alternate port number
    #[arg(long = "port")]
    pub port: Option<u16>,

    /// Specify alternate rsyncd.conf file
    #[arg(long = "config")]
    pub config: Option<PathBuf>,

    /// Read daemon-access password from FILE
    #[arg(long = "password-file")]
    pub password_file: Option<PathBuf>,

    // 動作制御オプション
    /// Perform a trial run with no changes made
    #[arg(short = 'n', long = "dry-run")]
    pub dry_run: bool,

    /// List the files instead of copying them
    #[arg(long = "list-only")]
    pub list_only: bool,

    /// Skip files that match in size
    #[arg(long = "size-only")]
    pub size_only: bool,

    /// Set I/O timeout in seconds
    #[arg(long = "timeout")]
    pub timeout: Option<u64>,

    // チェックサムオプション
    /// Choose checksum algorithm (md4, md5, xxh128)
    #[arg(long = "checksum-choice")]
    pub checksum_choice: Option<String>,
}

impl Cli {
    /// CLIからOptionsに変換
    pub fn into_options(self) -> Result<Options> {
        let mut options = Options::default();

        // 基本オプション
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

        // 転送オプション
        options.compress = self.compress;
        if let Some(algo) = self.compress_choice {
            options.compress_choice = Some(parse_compression_algorithm(&algo)?);
        }
        options.whole_file = self.whole_file;
        options.inplace = self.inplace;
        options.partial = self.partial;
        options.partial_dir = self.partial_dir;
        options.bwlimit = self.bwlimit;

        // バックアップオプション
        options.backup = self.backup;
        options.backup_dir = self.backup_dir;
        options.suffix = self.suffix;

        // 削除オプション
        options.delete = self.delete;
        options.delete_before = self.delete_before;
        options.delete_during = self.delete_during;
        options.delete_after = self.delete_after;
        options.delete_excluded = self.delete_excluded;
        options.remove_source_files = self.remove_source_files;

        // フィルタリングオプション
        options.exclude = self.exclude;
        options.include = self.include;
        options.exclude_from = self.exclude_from.into_iter().collect();
        options.include_from = self.include_from.into_iter().collect();
        options.files_from = self.files_from;

        // 出力・表示オプション
        options.progress = self.progress;
        options.itemize_changes = self.itemize_changes;
        options.stats = self.stats;
        options.human_readable = self.human_readable;
        options.log_file = self.log_file;

        // リモート転送オプション
        options.rsh = self.rsh;
        options.rsync_path = self.rsync_path;

        // デーモンモードオプション
        options.daemon = self.daemon;
        options.address = self.address;
        if let Some(port) = self.port {
            options.port = Some(port);
        }
        options.config = self.config;
        options.password_file = self.password_file;

        // 動作制御オプション
        options.dry_run = self.dry_run;
        options.list_only = self.list_only;
        options.size_only = self.size_only;
        options.timeout = self.timeout;

        // チェックサムオプション
        if let Some(algo) = self.checksum_choice {
            options.checksum_choice = Some(parse_checksum_algorithm(&algo)?);
        }

        // アーカイブモードの適用
        options.apply_archive_mode();

        // Windows非対応オプションの警告
        if self.perms {
            let warning = options.warn_unsupported_on_windows("perms");
            if !warning.is_empty() {
                eprintln!("{}", warning);
            }
        }
        if self.group {
            let warning = options.warn_unsupported_on_windows("group");
            if !warning.is_empty() {
                eprintln!("{}", warning);
            }
        }
        if self.owner {
            let warning = options.warn_unsupported_on_windows("owner");
            if !warning.is_empty() {
                eprintln!("{}", warning);
            }
        }
        if self.times {
            let warning = options.warn_unsupported_on_windows("times");
            if !warning.is_empty() {
                eprintln!("{}", warning);
            }
        }
        if self.devices_and_specials || self.devices || self.specials {
            let warning = options.warn_unsupported_on_windows("devices");
            if !warning.is_empty() {
                eprintln!("{}", warning);
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
