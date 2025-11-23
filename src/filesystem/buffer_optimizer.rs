




use std::path::Path;


pub struct BufferOptimizer {
    min_buffer_size: usize,
    max_buffer_size: usize,
    default_buffer_size: usize,
}

impl BufferOptimizer {

    pub fn new() -> Self {
        Self {
            min_buffer_size: 4 * 1024,
            max_buffer_size: 1024 * 1024,
            default_buffer_size: 64 * 1024,
        }
    }







    pub fn optimal_buffer_size(&self, file_size: u64) -> usize {
        if file_size < 64 * 1024 {

            self.min_buffer_size
        } else if file_size < 1024 * 1024 {

            self.default_buffer_size
        } else if file_size < 10 * 1024 * 1024 {

            256 * 1024
        } else if file_size < 100 * 1024 * 1024 {

            512 * 1024
        } else {

            self.max_buffer_size
        }
    }


    pub fn optimal_buffer_for_file(&self, file_path: &Path) -> usize {
        if let Ok(metadata) = std::fs::metadata(file_path) {
            self.optimal_buffer_size(metadata.len())
        } else {
            self.default_buffer_size
        }
    }


    #[allow(dead_code)]
    #[cfg(windows)]
    pub fn get_cluster_size(&self, path: &Path) -> Option<usize> {
        use windows::Win32::Storage::FileSystem::{
            GetDiskFreeSpaceW, GetVolumePathNameW,
        };


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


    #[allow(dead_code)]
    #[cfg(not(windows))]
    pub fn get_cluster_size(&self, _path: &Path) -> Option<usize> {
        Some(self.min_buffer_size)
    }


    #[allow(dead_code)]
    pub fn align_to_cluster(&self, buffer_size: usize, cluster_size: usize) -> usize {
        if cluster_size == 0 {
            return buffer_size;
        }


        ((buffer_size + cluster_size - 1) / cluster_size) * cluster_size
    }


    #[allow(dead_code)]
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


#[allow(dead_code)]
static GLOBAL_OPTIMIZER: std::sync::OnceLock<BufferOptimizer> = std::sync::OnceLock::new();


#[allow(dead_code)]
pub fn get_optimizer() -> &'static BufferOptimizer {
    GLOBAL_OPTIMIZER.get_or_init(BufferOptimizer::new)
}


#[allow(dead_code)]
pub fn optimal_buffer_size(file_path: &Path) -> usize {
    get_optimizer().optimal_buffer_with_alignment(file_path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_buffer_size_for_small_file() {
        let optimizer = BufferOptimizer::new();


        let size = optimizer.optimal_buffer_size(1024);
        assert_eq!(size, 4 * 1024);
    }

    #[test]
    fn test_buffer_size_for_medium_file() {
        let optimizer = BufferOptimizer::new();


        let size = optimizer.optimal_buffer_size(100 * 1024);
        assert_eq!(size, 64 * 1024);
    }

    #[test]
    fn test_buffer_size_for_large_file() {
        let optimizer = BufferOptimizer::new();


        let size = optimizer.optimal_buffer_size(5 * 1024 * 1024);
        assert_eq!(size, 256 * 1024);


        let size = optimizer.optimal_buffer_size(200 * 1024 * 1024);
        assert_eq!(size, 1024 * 1024);
    }

    #[test]
    fn test_cluster_alignment() {
        let optimizer = BufferOptimizer::new();


        let aligned = optimizer.align_to_cluster(10_000, 4096);
        assert_eq!(aligned, 12_288);


        let aligned = optimizer.align_to_cluster(8192, 4096);
        assert_eq!(aligned, 8192);
    }

    #[test]
    #[cfg(windows)]
    fn test_get_cluster_size() {
        let optimizer = BufferOptimizer::new();
        let temp_dir = std::env::temp_dir();


        if let Some(cluster_size) = optimizer.get_cluster_size(&temp_dir) {

            assert!(cluster_size >= 4096);
            assert!(cluster_size <= 64 * 1024);
        }
    }
}
