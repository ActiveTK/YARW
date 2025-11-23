use std::time::Duration;
use crate::output::VerboseOutput;

#[derive(Debug, Clone, Default)]
pub struct Stats {
    pub total_files: usize,
    pub total_bytes: u64,
    pub transferred_files: usize,
    pub transferred_bytes: u64,
    pub execution_time: Duration,
}

impl Stats {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn total_speed(&self) -> f64 {
        if self.execution_time.as_secs_f64() > 0.0 {
            self.transferred_bytes as f64 / self.execution_time.as_secs_f64()
        } else {
            0.0
        }
    }

    pub fn print(&self, verbose: &VerboseOutput) {
        verbose.print_basic(&format!("Total files: {}", self.total_files));
        verbose.print_basic(&format!("Total bytes: {}", self.total_bytes));
        verbose.print_basic(&format!("Transferred files: {}", self.transferred_files));
        verbose.print_basic(&format!("Transferred bytes: {}", self.transferred_bytes));
        verbose.print_basic(&format!("Execution time: {:?}", self.execution_time));
        verbose.print_basic(&format!("Total speed: {:.2} B/s", self.total_speed()));
    }
}
