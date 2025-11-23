use std::path::Path;
use globset::{Glob, GlobMatcher};
use crate::error::{Result, RsyncError};


#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PatternType {
    Include,
    Exclude,
}


#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MatchType {

    Wildcard,

    Directory,

    Absolute,
}


#[derive(Debug, Clone)]
pub struct FilterPattern {

    pub pattern: String,

    pub pattern_type: PatternType,

    pub match_type: MatchType,

    matcher: GlobMatcher,

    normalized_pattern: String,
}

impl FilterPattern {

    pub fn new(pattern: &str, pattern_type: PatternType) -> Result<Self> {

        let (normalized_pattern, match_type) = Self::parse_pattern(pattern);


        let glob = Glob::new(&normalized_pattern)
            .map_err(|e| RsyncError::InvalidPattern(format!("Invalid pattern '{}': {}", pattern, e)))?;

        Ok(Self {
            pattern: pattern.to_string(),
            pattern_type,
            match_type,
            matcher: glob.compile_matcher(),
            normalized_pattern: normalized_pattern.clone(),
        })
    }


    fn parse_pattern(pattern: &str) -> (String, MatchType) {
        let pattern = pattern.trim();


        if pattern.ends_with('/') {
            let dir_name = pattern.trim_end_matches('/');
            let dir_pattern = if pattern.starts_with('/') {

                let abs_dir = dir_name.trim_start_matches('/');
                format!("{}/**", abs_dir)
            } else {

                format!("**/{}{}", dir_name, "/**")
            };
            return (dir_pattern, MatchType::Directory);
        }


        if pattern.starts_with('/') {
            let abs_pattern = pattern.trim_start_matches('/');
            return (abs_pattern.to_string(), MatchType::Absolute);
        }





        let wildcard_pattern = if pattern.contains('/') {



            format!("{{,**/}}{}", pattern)
        } else {

            format!("**/{}", pattern)
        };

        (wildcard_pattern, MatchType::Wildcard)
    }


    pub fn matches(&self, path: &Path) -> bool {

        let path_str = path.to_string_lossy().replace('\\', "/");



        if self.match_type == MatchType::Directory {


            let pattern_str = &self.normalized_pattern;
            if let Some(dir_name) = pattern_str.strip_prefix("**/").and_then(|s| s.strip_suffix("/**")) {

                if path_str == dir_name || path_str.starts_with(&format!("{}/", dir_name)) {
                    return true;
                }

                if path_str.contains(&format!("/{}", dir_name)) {
                    let parts: Vec<&str> = path_str.split('/').collect();
                    if let Some(_pos) = parts.iter().position(|&p| p == dir_name) {

                        return true;
                    }
                }
            }
        }




        if self.pattern.contains('/') && !self.pattern.ends_with('/') && !self.pattern.starts_with('/') {

            let pattern_parts: Vec<&str> = self.pattern.split('/').collect();
            let path_parts: Vec<&str> = path_str.split('/').collect();


            if path_parts.len() >= pattern_parts.len() {
                for i in 0..=path_parts.len() - pattern_parts.len() {
                    let mut matches = true;
                    for (j, pattern_part) in pattern_parts.iter().enumerate() {
                        let path_part = path_parts[i + j];


                        if !glob_match_simple(pattern_part, path_part) {
                            matches = false;
                            break;
                        }
                    }

                    if matches {


                        if i + pattern_parts.len() == path_parts.len() {
                            return true;
                        }
                    }
                }
            }

            return false;
        }

        self.matcher.is_match(&path_str)
    }


    #[allow(dead_code)]
    pub fn is_directory_only(&self) -> bool {
        self.match_type == MatchType::Directory
    }
}


fn glob_match_simple(pattern: &str, text: &str) -> bool {

    let glob = match Glob::new(pattern) {
        Ok(g) => g,
        Err(_) => return false,
    };
    let matcher = glob.compile_matcher();
    matcher.is_match(text)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_wildcard_pattern() -> Result<()> {
        let pattern = FilterPattern::new("*.txt", PatternType::Exclude)?;

        assert!(pattern.matches(&PathBuf::from("file.txt")));
        assert!(pattern.matches(&PathBuf::from("dir/file.txt")));
        assert!(pattern.matches(&PathBuf::from("a/b/c/file.txt")));
        assert!(!pattern.matches(&PathBuf::from("file.dat")));

        Ok(())
    }

    #[test]
    fn test_directory_pattern() -> Result<()> {
        let pattern = FilterPattern::new("temp/", PatternType::Exclude)?;

        assert!(pattern.matches(&PathBuf::from("temp")));
        assert!(pattern.matches(&PathBuf::from("temp/file.txt")));
        assert!(pattern.matches(&PathBuf::from("temp/sub/file.txt")));

        Ok(())
    }

    #[test]
    fn test_absolute_pattern() -> Result<()> {
        let pattern = FilterPattern::new("/file.txt", PatternType::Exclude)?;

        assert!(pattern.matches(&PathBuf::from("file.txt")));
        assert!(!pattern.matches(&PathBuf::from("dir/file.txt")));

        Ok(())
    }

    #[test]
    fn test_specific_directory_pattern() -> Result<()> {
        let pattern = FilterPattern::new("dir/*.txt", PatternType::Exclude)?;

        assert!(pattern.matches(&PathBuf::from("dir/file.txt")));
        assert!(pattern.matches(&PathBuf::from("a/b/dir/file.txt")));
        assert!(!pattern.matches(&PathBuf::from("file.txt")));
        assert!(!pattern.matches(&PathBuf::from("dir/sub/file.txt")));

        Ok(())
    }

    #[test]
    fn test_doc_pattern() -> Result<()> {
        let pattern = FilterPattern::new("*.doc", PatternType::Exclude)?;

        assert!(pattern.matches(&PathBuf::from("document.doc")));
        assert!(pattern.matches(&PathBuf::from("dir/document.doc")));
        assert!(!pattern.matches(&PathBuf::from("document.txt")));

        Ok(())
    }
}
