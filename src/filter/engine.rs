use std::path::Path;
use std::fs::File;
use std::io::{BufRead, BufReader};
use crate::error::Result;
use super::pattern::{FilterPattern, PatternType};

/// フィルタエンジン
#[derive(Debug, Default)]
pub struct FilterEngine {
    patterns: Vec<FilterPattern>,
}

impl FilterEngine {
    /// 新しいフィルタエンジンを作成
    pub fn new() -> Self {
        Self {
            patterns: Vec::new(),
        }
    }

    /// excludeパターンを追加
    pub fn add_exclude(&mut self, pattern: &str) -> Result<()> {
        let filter = FilterPattern::new(pattern, PatternType::Exclude)?;
        self.patterns.push(filter);
        Ok(())
    }

    /// includeパターンを追加
    pub fn add_include(&mut self, pattern: &str) -> Result<()> {
        let filter = FilterPattern::new(pattern, PatternType::Include)?;
        self.patterns.push(filter);
        Ok(())
    }

    /// ファイルからexcludeパターンを読み込み
    pub fn add_exclude_from(&mut self, file_path: &Path) -> Result<()> {
        self.load_patterns_from_file(file_path, PatternType::Exclude)
    }

    /// ファイルからincludeパターンを読み込み
    pub fn add_include_from(&mut self, file_path: &Path) -> Result<()> {
        self.load_patterns_from_file(file_path, PatternType::Include)
    }

    /// ファイルからパターンを読み込み
    fn load_patterns_from_file(&mut self, file_path: &Path, pattern_type: PatternType) -> Result<()> {
        let file = File::open(file_path)?;
        let reader = BufReader::new(file);

        for line in reader.lines() {
            let line = line?;
            let line = line.trim();

            // 空行またはコメント行（#で始まる）をスキップ
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            // パターンを追加
            let filter = FilterPattern::new(line, pattern_type.clone())?;
            self.patterns.push(filter);
        }

        Ok(())
    }

    /// パスを含めるべきか判定
    ///
    /// rsyncのフィルタルール:
    /// 1. パターンは上から順に評価される
    /// 2. 最初にマッチしたパターンが適用される
    /// 3. includeパターンがマッチすれば含める
    /// 4. excludeパターンがマッチすれば除外
    /// 5. どのパターンにもマッチしなければ含める（デフォルト）
    pub fn should_include(&self, path: &Path) -> bool {
        // パターンが空なら全て含める
        if self.patterns.is_empty() {
            return true;
        }

        // パターンを順に評価
        for pattern in &self.patterns {
            if pattern.matches(path) {
                // 最初にマッチしたパターンを適用
                return match pattern.pattern_type {
                    PatternType::Include => true,
                    PatternType::Exclude => false,
                };
            }
        }

        // どのパターンにもマッチしなければ含める
        true
    }

    /// パターン数を取得
    pub fn pattern_count(&self) -> usize {
        self.patterns.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use tempfile::NamedTempFile;
    use std::io::Write;

    #[test]
    fn test_empty_engine() {
        let engine = FilterEngine::new();
        assert!(engine.should_include(&PathBuf::from("any/file.txt")));
    }

    #[test]
    fn test_exclude_pattern() -> Result<()> {
        let mut engine = FilterEngine::new();
        engine.add_exclude("*.txt")?;

        assert!(!engine.should_include(&PathBuf::from("file.txt")));
        assert!(!engine.should_include(&PathBuf::from("dir/file.txt")));
        assert!(engine.should_include(&PathBuf::from("file.dat")));

        Ok(())
    }

    #[test]
    fn test_include_pattern() -> Result<()> {
        let mut engine = FilterEngine::new();
        // 全てを除外してから特定のパターンを含める
        engine.add_include("*.txt")?;
        engine.add_exclude("*")?;

        // includeが先にマッチするので含まれる
        assert!(engine.should_include(&PathBuf::from("file.txt")));
        // excludeがマッチするので除外される
        assert!(!engine.should_include(&PathBuf::from("file.dat")));

        Ok(())
    }

    #[test]
    fn test_pattern_order() -> Result<()> {
        let mut engine = FilterEngine::new();
        // 順序が重要：最初にマッチしたパターンが適用される
        engine.add_exclude("*.txt")?;
        engine.add_include("important.txt")?;

        // *.txt が先にマッチするので除外される
        assert!(!engine.should_include(&PathBuf::from("important.txt")));
        assert!(!engine.should_include(&PathBuf::from("file.txt")));

        Ok(())
    }

    #[test]
    fn test_reverse_pattern_order() -> Result<()> {
        let mut engine = FilterEngine::new();
        // includeを先に追加
        engine.add_include("important.txt")?;
        engine.add_exclude("*.txt")?;

        // important.txt が先にマッチするので含まれる
        assert!(engine.should_include(&PathBuf::from("important.txt")));
        // *.txt がマッチするので除外される
        assert!(!engine.should_include(&PathBuf::from("other.txt")));

        Ok(())
    }

    #[test]
    fn test_load_from_file() -> Result<()> {
        // 一時ファイルを作成
        let mut temp_file = NamedTempFile::new()?;
        writeln!(temp_file, "# Comment line")?;
        writeln!(temp_file)?; // 空行
        writeln!(temp_file, "*.txt")?;
        writeln!(temp_file, "*.log")?;
        writeln!(temp_file, "# Another comment")?;
        writeln!(temp_file, "temp/")?;
        temp_file.flush()?;

        let mut engine = FilterEngine::new();
        engine.add_exclude_from(temp_file.path())?;

        // パターンが正しく読み込まれているか確認
        assert_eq!(engine.pattern_count(), 3);

        // パターンがマッチするか確認
        assert!(!engine.should_include(&PathBuf::from("file.txt")));
        assert!(!engine.should_include(&PathBuf::from("file.log")));
        assert!(!engine.should_include(&PathBuf::from("temp/file.dat")));
        assert!(engine.should_include(&PathBuf::from("file.dat")));

        Ok(())
    }

    #[test]
    fn test_directory_exclusion() -> Result<()> {
        let mut engine = FilterEngine::new();
        engine.add_exclude(".git/")?;
        engine.add_exclude("node_modules/")?;

        assert!(!engine.should_include(&PathBuf::from(".git")));
        assert!(!engine.should_include(&PathBuf::from(".git/config")));
        assert!(!engine.should_include(&PathBuf::from("node_modules")));
        assert!(!engine.should_include(&PathBuf::from("node_modules/package/index.js")));
        assert!(engine.should_include(&PathBuf::from("src/main.rs")));

        Ok(())
    }
}
