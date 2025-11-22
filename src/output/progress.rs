use indicatif::{ProgressBar, ProgressStyle};

/// プログレス表示
pub struct ProgressDisplay {
    bar: ProgressBar,
    #[allow(dead_code)]
    total_bytes: u64,
}

impl ProgressDisplay {
    /// 新しいプログレス表示を作成
    pub fn new(total_bytes: u64, file_count: usize) -> Self {
        let bar = ProgressBar::new(total_bytes);

        bar.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({percent}%) {msg}")
                .expect("Invalid progress bar template")
                .progress_chars("#>-")
        );

        bar.set_message(format!("{} files", file_count));

        Self {
            bar,
            total_bytes,
        }
    }

    /// 進捗を更新
    pub fn update(&self, bytes_transferred: u64, current_file: &str) {
        self.bar.set_position(bytes_transferred);
        self.bar.set_message(current_file.to_string());
    }

    /// 転送完了
    pub fn finish(&self) {
        self.bar.finish_with_message("Transfer complete");
    }

    /// プログレスバーを非表示にする（テストやクワイエットモード用）
    #[allow(dead_code)]
    pub fn hide(&self) {
        self.bar.set_draw_target(indicatif::ProgressDrawTarget::hidden());
    }
}

impl Drop for ProgressDisplay {
    fn drop(&mut self) {
        if !self.bar.is_finished() {
            self.bar.finish_and_clear();
        }
    }
}
