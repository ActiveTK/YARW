//! Verbose出力モジュール
//!
//! 冗長レベルに応じた詳細な情報出力を管理する。
//! -v: 基本的な情報
//! -vv: より詳細な情報
//! -vvv: デバッグレベルの情報

use std::path::Path;
use crate::filesystem::FileInfo;

/// Verbose出力マネージャー
#[allow(dead_code)]
pub struct VerboseOutput {
    /// 冗長レベル (0=quiet, 1=normal, 2=verbose, 3+=very verbose)
    level: u8,
    /// 静かモード
    quiet: bool,
}

impl VerboseOutput {
    /// 新しいVerboseOutputを作成
    pub fn new(level: u8, quiet: bool) -> Self {
        VerboseOutput { level, quiet }
    }

    /// レベル1以上の場合に出力
    pub fn print_basic<S: AsRef<str>>(&self, message: S) {
        if !self.quiet && self.level >= 1 {
            println!("{}", message.as_ref());
        }
    }

    /// レベル2以上の場合に出力
    pub fn print_verbose<S: AsRef<str>>(&self, message: S) {
        if !self.quiet && self.level >= 2 {
            println!("{}", message.as_ref());
        }
    }

    /// レベル3以上の場合に出力
    pub fn print_debug<S: AsRef<str>>(&self, message: S) {
        if !self.quiet && self.level >= 3 {
            println!("[DEBUG] {}", message.as_ref());
        }
    }

    /// エラーメッセージを出力（quietモードでも表示）
    #[allow(dead_code)]
    pub fn print_error<S: AsRef<str>>(&self, message: S) {
        eprintln!("Error: {}", message.as_ref());
    }

    /// 警告メッセージを出力（quietモードでも表示）
    #[allow(dead_code)]
    pub fn print_warning<S: AsRef<str>>(&self, message: S) {
        eprintln!("Warning: {}", message.as_ref());
    }

    /// ファイル転送開始を通知
    pub fn print_file_start(&self, file_info: &FileInfo) {
        if !self.quiet && self.level >= 1 {
            println!("{}", file_info.path.display());
        }
    }

    /// ファイル転送完了を通知（詳細モード）
    pub fn print_file_complete(&self, file_info: &FileInfo, bytes_transferred: u64) {
        if !self.quiet && self.level >= 2 {
            println!(
                "  {} ({} bytes transferred)",
                file_info.path.display(),
                bytes_transferred
            );
        }
    }

    /// ディレクトリスキャン開始を通知
    pub fn print_scan_start(&self, path: &Path) {
        if !self.quiet && self.level >= 2 {
            println!("Scanning directory: {}", path.display());
        }
    }

    /// ディレクトリスキャン完了を通知
    pub fn print_scan_complete(&self, path: &Path, file_count: usize) {
        if !self.quiet && self.level >= 2 {
            println!(
                "Scan complete: {} ({} files)",
                path.display(),
                file_count
            );
        }
    }

    /// ファイル削除を通知
    pub fn print_delete(&self, path: &Path) {
        if !self.quiet && self.level >= 1 {
            println!("deleting {}", path.display());
        }
    }

    /// ファイルスキップを通知
    pub fn print_skip(&self, path: &Path, reason: &str) {
        if !self.quiet && self.level >= 2 {
            println!("skipping {} ({})", path.display(), reason);
        }
    }

    /// チェックサム計算開始を通知
    pub fn print_checksum_start(&self, path: &Path) {
        if !self.quiet && self.level >= 3 {
            println!("[DEBUG] Computing checksum for {}", path.display());
        }
    }

    /// デルタ計算開始を通知
    pub fn print_delta_start(&self, path: &Path, block_count: usize) {
        if !self.quiet && self.level >= 3 {
            println!(
                "[DEBUG] Computing delta for {} ({} blocks)",
                path.display(),
                block_count
            );
        }
    }

    /// 圧縮情報を通知
    pub fn print_compression(&self, original_size: u64, compressed_size: u64) {
        if !self.quiet && self.level >= 2 {
            let ratio = if original_size > 0 {
                (compressed_size as f64 / original_size as f64) * 100.0
            } else {
                0.0
            };
            println!(
                "Compression: {} -> {} bytes ({:.1}%)",
                original_size, compressed_size, ratio
            );
        }
    }

    /// 転送速度を通知
    pub fn print_transfer_rate(&self, bytes: u64, duration_secs: f64) {
        if !self.quiet && self.level >= 2 {
            let rate = if duration_secs > 0.0 {
                bytes as f64 / duration_secs / 1024.0 / 1024.0
            } else {
                0.0
            };
            println!("Transfer rate: {:.2} MB/s", rate);
        }
    }

    /// プロトコルバージョン交渉を通知
    pub fn print_protocol_version(&self, local: u32, remote: u32, negotiated: u32) {
        if !self.quiet && self.level >= 3 {
            println!(
                "[DEBUG] Protocol version: local={}, remote={}, negotiated={}",
                local, remote, negotiated
            );
        }
    }

    /// SSH接続情報を通知
    pub fn print_ssh_connect(&self, host: &str, port: u16) {
        if !self.quiet && self.level >= 2 {
            println!("Connecting to {}:{}...", host, port);
        }
    }

    /// SSH認証成功を通知
    pub fn print_ssh_auth_success(&self, method: &str) {
        if !self.quiet && self.level >= 2 {
            println!("Authentication successful ({})", method);
        }
    }

    /// ドライラン通知
    pub fn print_dry_run_notice(&self) {
        if !self.quiet {
            println!("*** DRY RUN MODE - No files will be modified ***");
        }
    }

    /// バックアップ作成通知
    pub fn print_backup(&self, original: &Path, backup: &Path) {
        if !self.quiet && self.level >= 1 {
            println!(
                "Backing up {} to {}",
                original.display(),
                backup.display()
            );
        }
    }

    /// リモートコマンド実行を通知
    pub fn print_remote_command(&self, command: &str) {
        if !self.quiet && self.level >= 3 {
            println!("[DEBUG] Executing remote command: {}", command);
        }
    }

    /// 現在のレベルを取得
    pub fn level(&self) -> u8 {
        self.level
    }

    /// quietモードかどうか
    pub fn is_quiet(&self) -> bool {
        self.quiet
    }
}

impl Default for VerboseOutput {
    fn default() -> Self {
        VerboseOutput {
            level: 0,
            quiet: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_verbose_levels() {
        let v0 = VerboseOutput::new(0, false);
        let v1 = VerboseOutput::new(1, false);
        let v2 = VerboseOutput::new(2, false);
        let v3 = VerboseOutput::new(3, false);

        assert_eq!(v0.level(), 0);
        assert_eq!(v1.level(), 1);
        assert_eq!(v2.level(), 2);
        assert_eq!(v3.level(), 3);
    }

    #[test]
    fn test_quiet_mode() {
        let quiet = VerboseOutput::new(1, true);
        assert!(quiet.is_quiet());

        let not_quiet = VerboseOutput::new(1, false);
        assert!(!not_quiet.is_quiet());
    }

    #[test]
    fn test_default() {
        let default = VerboseOutput::default();
        assert_eq!(default.level(), 0);
        assert!(!default.is_quiet());
    }
}
