use std::path::Path;
use std::fs::File;
use std::io::{BufRead, BufReader};
use crate::error::Result;
use super::pattern::{FilterPattern, PatternType};


#[derive(Debug, Default)]
pub struct FilterEngine {
    patterns: Vec<FilterPattern>,
}

impl FilterEngine {

    pub fn new() -> Self {
        Self {
            patterns: Vec::new(),
        }
    }


    pub fn add_exclude(&mut self, pattern: &str) -> Result<()> {
        let filter = FilterPattern::new(pattern, PatternType::Exclude)?;
        self.patterns.push(filter);
        Ok(())
    }


    pub fn add_include(&mut self, pattern: &str) -> Result<()> {
        let filter = FilterPattern::new(pattern, PatternType::Include)?;
        self.patterns.push(filter);
        Ok(())
    }


    pub fn add_exclude_from(&mut self, file_path: &Path) -> Result<()> {
        self.load_patterns_from_file(file_path, PatternType::Exclude)
    }


    pub fn add_include_from(&mut self, file_path: &Path) -> Result<()> {
        self.load_patterns_from_file(file_path, PatternType::Include)
    }


    fn load_patterns_from_file(&mut self, file_path: &Path, pattern_type: PatternType) -> Result<()> {
        let file = File::open(file_path)?;
        let reader = BufReader::new(file);

        for line in reader.lines() {
            let line = line?;
            let line = line.trim();


            if line.is_empty() || line.starts_with('#') {
                continue;
            }


            let filter = FilterPattern::new(line, pattern_type.clone())?;
            self.patterns.push(filter);
        }

        Ok(())
    }









    pub fn should_include(&self, path: &Path) -> bool {

        if self.patterns.is_empty() {
            return true;
        }


        for pattern in &self.patterns {
            if pattern.matches(path) {

                return match pattern.pattern_type {
                    PatternType::Include => true,
                    PatternType::Exclude => false,
                };
            }
        }


        true
    }


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

        engine.add_include("*.txt")?;
        engine.add_exclude("*")?;


        assert!(engine.should_include(&PathBuf::from("file.txt")));

        assert!(!engine.should_include(&PathBuf::from("file.dat")));

        Ok(())
    }

    #[test]
    fn test_pattern_order() -> Result<()> {
        let mut engine = FilterEngine::new();

        engine.add_exclude("*.txt")?;
        engine.add_include("important.txt")?;


        assert!(!engine.should_include(&PathBuf::from("important.txt")));
        assert!(!engine.should_include(&PathBuf::from("file.txt")));

        Ok(())
    }

    #[test]
    fn test_reverse_pattern_order() -> Result<()> {
        let mut engine = FilterEngine::new();

        engine.add_include("important.txt")?;
        engine.add_exclude("*.txt")?;


        assert!(engine.should_include(&PathBuf::from("important.txt")));

        assert!(!engine.should_include(&PathBuf::from("other.txt")));

        Ok(())
    }

    #[test]
    fn test_load_from_file() -> Result<()> {

        let mut temp_file = NamedTempFile::new()?;
        writeln!(temp_file, "# Comment line")?;
        writeln!(temp_file)?;
        writeln!(temp_file, "*.txt")?;
        writeln!(temp_file, "*.log")?;
        writeln!(temp_file, "# Another comment")?;
        writeln!(temp_file, "temp/")?;
        temp_file.flush()?;

        let mut engine = FilterEngine::new();
        engine.add_exclude_from(temp_file.path())?;


        assert_eq!(engine.pattern_count(), 3);


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
