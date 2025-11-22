use std::time::{Duration, Instant};

pub struct BandwidthLimiter {
    limit: u64, // bytes per second
    start_time: Instant,
    bytes_sent: u64,
}

impl BandwidthLimiter {
    pub fn new(limit: u64) -> Self {
        BandwidthLimiter {
            limit,
            start_time: Instant::now(),
            bytes_sent: 0,
        }
    }

    pub fn limit(&mut self, bytes: u64) {
        self.bytes_sent += bytes;
        let elapsed = self.start_time.elapsed();
        let expected_time = Duration::from_secs_f64(self.bytes_sent as f64 / self.limit as f64);
        if expected_time > elapsed {
            let delay = expected_time - elapsed;
            std::thread::sleep(delay);
        }
    }
}
