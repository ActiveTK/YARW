/// バッファサイズ動的最適化モジュール
///
/// ファイルサイズ、ファイルシステムのクラスタサイズ、
/// ストレージ特性に応じて最適なバッファサイズを決定します。

use std::path::Path;

/// バッファサイズ最適化エンジン
pub struct BufferOptimizer {
    /// 最小バッファサイズ（4KB - NTFSデフォルトクラスタサイズ）
    min_buffer_size: usize,
    /// 最大バッファサイズ（1MB）
    max_buffer_size: usize,
    /// デフォルトバッファサイズ（64KB - 一般的な最適値）
    default_buffer_size: usize,
}

impl BufferOptimizer {
    /// 新しいオプティマイザーを作成
    pub fn new() -> Self {
        Self {
            min_buffer_size: 4 * 1024,       // 4KB
            max_buffer_size: 1024 * 1024,    // 1MB
            default_buffer_size: 64 * 1024,  // 64KB
        }
    }

    /// ファイルサイズに基づいて最適なバッファサイズを計算
    ///
    /// # 戦略
    /// - 小ファイル（<64KB）: 4KB（クラスタサイズ）
    /// - 中ファイル（64KB-1MB）: 64KB（バランス重視）
    /// - 大ファイル（>1MB）: 256KB-1MB（スループット重視）
    pub fn optimal_buffer_size(&self, file_size: u64) -> usize {
        if file_size < 64 * 1024 {
            // 小ファイル: クラスタサイズ
            self.min_buffer_size
        } else if file_size < 1024 * 1024 {
            // 中ファイル: デフォルト
            self.default_buffer_size
        } else if file_size < 10 * 1024 * 1024 {
            // 大ファイル（<10MB）: 256KB
            256 * 1024
        } else if file_size < 100 * 1024 * 1024 {
            // 大ファイル（<100MB）: 512KB
            512 * 1024
        } else {
            // 超大ファイル（>=100MB）: 1MB
            self.max_buffer_size
        }
    }

    /// ファイルパスから最適なバッファサイズを取得
    pub fn optimal_buffer_for_file(&self, file_path: &Path) -> usize {
        if let Ok(metadata) = std::fs::metadata(file_path) {
            self.optimal_buffer_size(metadata.len())
        } else {
            self.default_buffer_size
        }
    }

    /// Windows NTFSクラスタサイズの取得（Windows API使用）
    #[cfg(windows)]
    pub fn get_cluster_size(&self, path: &Path) -> Option<usize> {
        use windows::Win32::Storage::FileSystem::{
            GetDiskFreeSpaceW, GetVolumePathNameW,
        };

        // パスからボリューム名を取得
        let path_str = path.to_str()?;
        let path_wide: Vec<u16> = path_str.encode_utf16().chain(std::iter::once(0)).collect();

        let mut volume_path = vec![0u16; 260];
        let result = unsafe {
            GetVolumePathNameW(
                windows::core::PCWSTR(path_wide.as_ptr()),
                &mut volume_path,
            )
        };

        if result.is_err() {
            return None;
        }

        // クラスタサイズを取得
        let mut sectors_per_cluster = 0u32;
        let mut bytes_per_sector = 0u32;
        let mut _number_of_free_clusters = 0u32;
        let mut _total_number_of_clusters = 0u32;

        let result = unsafe {
            GetDiskFreeSpaceW(
                windows::core::PCWSTR(volume_path.as_ptr()),
                Some(&mut sectors_per_cluster),
                Some(&mut bytes_per_sector),
                Some(&mut _number_of_free_clusters),
                Some(&mut _total_number_of_clusters),
            )
        };

        if result.is_ok() {
            Some((sectors_per_cluster * bytes_per_sector) as usize)
        } else {
            None
        }
    }

    /// 非Windows環境でのダミー実装
    #[cfg(not(windows))]
    pub fn get_cluster_size(&self, _path: &Path) -> Option<usize> {
        Some(self.min_buffer_size)
    }

    /// クラスタサイズに合わせてバッファサイズをアライメント
    pub fn align_to_cluster(&self, buffer_size: usize, cluster_size: usize) -> usize {
        if cluster_size == 0 {
            return buffer_size;
        }

        // クラスタサイズの倍数に切り上げ
        ((buffer_size + cluster_size - 1) / cluster_size) * cluster_size
    }

    /// 最適なバッファサイズを計算（クラスタアライメント付き）
    pub fn optimal_buffer_with_alignment(&self, file_path: &Path) -> usize {
        let base_size = self.optimal_buffer_for_file(file_path);

        if let Some(cluster_size) = self.get_cluster_size(file_path) {
            self.align_to_cluster(base_size, cluster_size)
        } else {
            base_size
        }
    }
}

impl Default for BufferOptimizer {
    fn default() -> Self {
        Self::new()
    }
}

/// グローバルオプティマイザーインスタンス
static GLOBAL_OPTIMIZER: std::sync::OnceLock<BufferOptimizer> = std::sync::OnceLock::new();

/// グローバルオプティマイザーを取得
pub fn get_optimizer() -> &'static BufferOptimizer {
    GLOBAL_OPTIMIZER.get_or_init(BufferOptimizer::new)
}

/// ファイルに最適なバッファサイズを取得（ヘルパー関数）
pub fn optimal_buffer_size(file_path: &Path) -> usize {
    get_optimizer().optimal_buffer_with_alignment(file_path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_buffer_size_for_small_file() {
        let optimizer = BufferOptimizer::new();

        // 小ファイル（1KB）
        let size = optimizer.optimal_buffer_size(1024);
        assert_eq!(size, 4 * 1024); // 4KB
    }

    #[test]
    fn test_buffer_size_for_medium_file() {
        let optimizer = BufferOptimizer::new();

        // 中ファイル（100KB）
        let size = optimizer.optimal_buffer_size(100 * 1024);
        assert_eq!(size, 64 * 1024); // 64KB
    }

    #[test]
    fn test_buffer_size_for_large_file() {
        let optimizer = BufferOptimizer::new();

        // 大ファイル（5MB）
        let size = optimizer.optimal_buffer_size(5 * 1024 * 1024);
        assert_eq!(size, 256 * 1024); // 256KB

        // 超大ファイル（200MB）
        let size = optimizer.optimal_buffer_size(200 * 1024 * 1024);
        assert_eq!(size, 1024 * 1024); // 1MB
    }

    #[test]
    fn test_cluster_alignment() {
        let optimizer = BufferOptimizer::new();

        // 4KBクラスタサイズへのアライメント
        let aligned = optimizer.align_to_cluster(10_000, 4096);
        assert_eq!(aligned, 12_288); // 3 * 4096

        // 既にアライメント済み
        let aligned = optimizer.align_to_cluster(8192, 4096);
        assert_eq!(aligned, 8192); // 2 * 4096
    }

    #[test]
    #[cfg(windows)]
    fn test_get_cluster_size() {
        let optimizer = BufferOptimizer::new();
        let temp_dir = std::env::temp_dir();

        // クラスタサイズを取得（通常は4096）
        if let Some(cluster_size) = optimizer.get_cluster_size(&temp_dir) {
            // NTFSの一般的なクラスタサイズは4KB
            assert!(cluster_size >= 4096);
            assert!(cluster_size <= 64 * 1024); // 最大64KB
        }
    }
}
