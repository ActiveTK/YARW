use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::Path;
use std::sync::{Arc, Mutex};
use crate::error::Result;




#[derive(Clone)]
pub struct Logger {
    file: Arc<Mutex<File>>,
}

impl Logger {







    pub fn new(log_path: &Path) -> Result<Self> {
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(log_path)?;

        Ok(Self {
            file: Arc::new(Mutex::new(file)),
        })
    }





    pub fn log(&self, message: &str) -> Result<()> {
        let mut file = self.file.lock().unwrap();
        writeln!(file, "{}", message)?;
        file.flush()?;
        Ok(())
    }





    pub fn log_with_timestamp(&self, message: &str) -> Result<()> {
        let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
        self.log(&format!("[{}] {}", timestamp, message))
    }
}


static GLOBAL_LOGGER: Mutex<Option<Logger>> = Mutex::new(None);


pub fn init_logger(log_path: &Path) -> Result<()> {
    let logger = Logger::new(log_path)?;
    let mut global = GLOBAL_LOGGER.lock().unwrap();
    *global = Some(logger);
    Ok(())
}





pub fn log(message: &str) {
    if let Some(logger) = GLOBAL_LOGGER.lock().unwrap().as_ref() {
        let _ = logger.log(message);
    }
}





pub fn log_with_timestamp(message: &str) {
    if let Some(logger) = GLOBAL_LOGGER.lock().unwrap().as_ref() {
        let _ = logger.log_with_timestamp(message);
    }
}


pub fn is_logging_enabled() -> bool {
    GLOBAL_LOGGER.lock().unwrap().is_some()
}


#[macro_export]
macro_rules! rsync_log {
    ($($arg:tt)*) => {
        if $crate::output::logger::is_logging_enabled() {
            $crate::output::logger::log(&format!($($arg)*));
        }
    };
}


#[macro_export]
macro_rules! rsync_log_ts {
    ($($arg:tt)*) => {
        if $crate::output::logger::is_logging_enabled() {
            $crate::output::logger::log_with_timestamp(&format!($($arg)*));
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;
    use std::io::Read;

    #[test]
    fn test_logger_basic() -> Result<()> {
        let temp_file = NamedTempFile::new()?;
        let logger = Logger::new(temp_file.path())?;

        logger.log("Test message 1")?;
        logger.log("Test message 2")?;


        let mut file = File::open(temp_file.path())?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)?;

        assert!(contents.contains("Test message 1"));
        assert!(contents.contains("Test message 2"));

        Ok(())
    }

    #[test]
    fn test_global_logger() -> Result<()> {
        let temp_file = NamedTempFile::new()?;
        init_logger(temp_file.path())?;

        log("Global log message");


        let mut file = File::open(temp_file.path())?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)?;

        assert!(contents.contains("Global log message"));

        Ok(())
    }
}
