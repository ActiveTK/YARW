use std::time::Duration;

#[allow(dead_code)]
#[derive(Debug, Clone, Default)]
pub struct Stats {
    pub total_files: usize,
    pub total_bytes: u64,
    pub transferred_files: usize,
    pub transferred_bytes: u64,
    pub execution_time: Duration,
}

impl Stats {
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self::default()
    }

    #[allow(dead_code)]
    pub fn total_speed(&self) -> f64 {
        if self.execution_time.as_secs_f64() > 0.0 {
            self.transferred_bytes as f64 / self.execution_time.as_secs_f64()
        } else {
            0.0
        }
    }

    #[allow(dead_code)]
    pub fn print(&self) {
        println!("Total files: {}", self.total_files);
        println!("Total bytes: {}", self.total_bytes);
        println!("Transferred files: {}", self.transferred_files);
        println!("Transferred bytes: {}", self.transferred_bytes);
        println!("Execution time: {:?}", self.execution_time);
        println!("Total speed: {:.2} B/s", self.total_speed());
    }
}
