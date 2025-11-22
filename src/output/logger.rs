use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::Path;
use std::sync::{Arc, Mutex};
use crate::error::Result;

/// ログファイルハンドラー
///
/// `--log-file` オプションで指定されたファイルに操作ログを記録します。
#[derive(Clone)]
pub struct Logger {
    file: Arc<Mutex<File>>,
}

impl Logger {
    /// 新しいロガーを作成
    ///
    /// # 引数
    /// * `log_path` - ログファイルのパス
    ///
    /// # エラー
    /// ファイルの作成に失敗した場合はエラーを返します
    pub fn new(log_path: &Path) -> Result<Self> {
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(log_path)?;

        Ok(Self {
            file: Arc::new(Mutex::new(file)),
        })
    }

    /// ログメッセージを記録
    ///
    /// # 引数
    /// * `message` - 記録するメッセージ
    pub fn log(&self, message: &str) -> Result<()> {
        let mut file = self.file.lock().unwrap();
        writeln!(file, "{}", message)?;
        file.flush()?;
        Ok(())
    }

    /// タイムスタンプ付きでログメッセージを記録
    ///
    /// # 引数
    /// * `message` - 記録するメッセージ
    pub fn log_with_timestamp(&self, message: &str) -> Result<()> {
        let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
        self.log(&format!("[{}] {}", timestamp, message))
    }
}

/// グローバルロガーインスタンス
static GLOBAL_LOGGER: Mutex<Option<Logger>> = Mutex::new(None);

/// グローバルロガーを初期化
pub fn init_logger(log_path: &Path) -> Result<()> {
    let logger = Logger::new(log_path)?;
    let mut global = GLOBAL_LOGGER.lock().unwrap();
    *global = Some(logger);
    Ok(())
}

/// グローバルロガーにログを記録
///
/// # 引数
/// * `message` - 記録するメッセージ
pub fn log(message: &str) {
    if let Some(logger) = GLOBAL_LOGGER.lock().unwrap().as_ref() {
        let _ = logger.log(message);
    }
}

/// タイムスタンプ付きでグローバルロガーにログを記録
///
/// # 引数
/// * `message` - 記録するメッセージ
pub fn log_with_timestamp(message: &str) {
    if let Some(logger) = GLOBAL_LOGGER.lock().unwrap().as_ref() {
        let _ = logger.log_with_timestamp(message);
    }
}

/// ログが有効かどうかを確認
pub fn is_logging_enabled() -> bool {
    GLOBAL_LOGGER.lock().unwrap().is_some()
}

/// ログマクロ - ログが有効な場合にのみ記録
#[macro_export]
macro_rules! rsync_log {
    ($($arg:tt)*) => {
        if $crate::output::logger::is_logging_enabled() {
            $crate::output::logger::log(&format!($($arg)*));
        }
    };
}

/// タイムスタンプ付きログマクロ
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

        // ファイルの内容を確認
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

        // ファイルの内容を確認
        let mut file = File::open(temp_file.path())?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)?;

        assert!(contents.contains("Global log message"));

        Ok(())
    }
}
