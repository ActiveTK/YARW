use std::path::Path;
use globset::{Glob, GlobMatcher};
use crate::error::{Result, RsyncError};

/// パターンのタイプ（includeまたはexclude）
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PatternType {
    Include,
    Exclude,
}

/// マッチングのタイプ
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MatchType {
    /// ワイルドカードパターン（*.txt, file?.dat など）
    Wildcard,
    /// ディレクトリパターン（dir/, path/to/dir/ など）
    Directory,
    /// 絶対パスパターン（/path/to/file など）
    Absolute,
}

/// フィルタパターン
#[derive(Debug, Clone)]
pub struct FilterPattern {
    /// 元のパターン文字列
    pub pattern: String,
    /// パターンタイプ（includeまたはexclude）
    pub pattern_type: PatternType,
    /// マッチングタイプ
    pub match_type: MatchType,
    /// Globマッチャー
    matcher: GlobMatcher,
    /// 正規化されたパターン（内部使用）
    normalized_pattern: String,
}

impl FilterPattern {
    /// 新しいフィルタパターンを作成
    pub fn new(pattern: &str, pattern_type: PatternType) -> Result<Self> {
        // パターンの解析
        let (normalized_pattern, match_type) = Self::parse_pattern(pattern);

        // Globパターンをコンパイル
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

    /// パターンを解析して正規化
    fn parse_pattern(pattern: &str) -> (String, MatchType) {
        let pattern = pattern.trim();

        // ディレクトリパターン（末尾が / の場合）
        if pattern.ends_with('/') {
            let dir_name = pattern.trim_end_matches('/');
            let dir_pattern = if pattern.starts_with('/') {
                // 絶対パス: /path/to/dir/
                let abs_dir = dir_name.trim_start_matches('/');
                format!("{}/**", abs_dir)
            } else {
                // 相対パス: dir/ -> **/dir または **/dir/**
                format!("**/{}{}", dir_name, "/**")
            };
            return (dir_pattern, MatchType::Directory);
        }

        // 絶対パスパターン
        if pattern.starts_with('/') {
            let abs_pattern = pattern.trim_start_matches('/');
            return (abs_pattern.to_string(), MatchType::Absolute);
        }

        // ワイルドカードパターン（相対パス）
        // "*.txt" -> "**/*.txt" (任意の場所にマッチ)
        // "dir/*.txt" -> "dir/*.txt" または "**/dir/*.txt"
        // rsyncの挙動: "dir/*.txt" は dir/ 直下のファイルのみ、サブディレクトリは含まない
        let wildcard_pattern = if pattern.contains('/') {
            // パスを含むパターンの場合、そのまま使う（ルートからのパスとして扱う）
            // ただし、任意の階層でマッチさせるために **/をつける
            // ただし、globsetでは * は / を越えないので、dir/*.txt は dir/ 直下のみマッチする
            format!("{{,**/}}{}", pattern)
        } else {
            // 単純なワイルドカード: "*.txt" -> "**/*.txt"
            format!("**/{}", pattern)
        };

        (wildcard_pattern, MatchType::Wildcard)
    }

    /// パスがこのパターンにマッチするか判定
    pub fn matches(&self, path: &Path) -> bool {
        // Windowsのパス区切り文字を / に変換
        let path_str = path.to_string_lossy().replace('\\', "/");

        // ディレクトリパターンの特別処理
        // "temp/" は "temp" と "temp/**" の両方にマッチすべき
        if self.match_type == MatchType::Directory {
            // パターンから **/dir/** の形式を抽出
            // 例: "**/temp/**" -> "temp"
            let pattern_str = &self.normalized_pattern;
            if let Some(dir_name) = pattern_str.strip_prefix("**/").and_then(|s| s.strip_suffix("/**")) {
                // パスがディレクトリ名と完全一致、またはディレクトリ名で始まるか確認
                if path_str == dir_name || path_str.starts_with(&format!("{}/", dir_name)) {
                    return true;
                }
                // さらに深い階層でもマッチ: "a/b/temp" or "a/b/temp/file"
                if path_str.contains(&format!("/{}", dir_name)) {
                    let parts: Vec<&str> = path_str.split('/').collect();
                    if let Some(_pos) = parts.iter().position(|&p| p == dir_name) {
                        // temp が見つかった場合、それ以降は全てマッチ
                        return true;
                    }
                }
            }
        }

        // パターンに / が含まれる場合（例: "dir/*.txt"）の特別処理
        // ただし、絶対パターン（/で始まる）やディレクトリパターン（/で終わる）は除外
        // 元のパターンから判断
        if self.pattern.contains('/') && !self.pattern.ends_with('/') && !self.pattern.starts_with('/') {
            // パターンを分解: "dir/*.txt" -> ["dir", "*.txt"]
            let pattern_parts: Vec<&str> = self.pattern.split('/').collect();
            let path_parts: Vec<&str> = path_str.split('/').collect();

            // パスの中から pattern_parts と一致する連続部分を探す
            if path_parts.len() >= pattern_parts.len() {
                for i in 0..=path_parts.len() - pattern_parts.len() {
                    let mut matches = true;
                    for (j, pattern_part) in pattern_parts.iter().enumerate() {
                        let path_part = path_parts[i + j];

                        // ワイルドカードマッチング
                        if !glob_match_simple(pattern_part, path_part) {
                            matches = false;
                            break;
                        }
                    }

                    if matches {
                        // 完全一致が見つかった
                        // さらに、これがパスの終端である必要がある（サブディレクトリを含まない）
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

    /// パターンがディレクトリ専用かどうか
    #[allow(dead_code)]
    pub fn is_directory_only(&self) -> bool {
        self.match_type == MatchType::Directory
    }
}

/// 簡易的なglobマッチング（単一パス要素に対して）
fn glob_match_simple(pattern: &str, text: &str) -> bool {
    // * や ? を含むパターンマッチング
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
