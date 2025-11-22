/// SSH コマンドパーサー
///
/// -e オプションで指定されたSSHコマンドを解析し、
/// 接続パラメータを抽出します。

use std::path::PathBuf;

/// SSH接続パラメータ
#[derive(Debug, Clone)]
pub struct SshConnectionParams {
    /// ポート番号
    pub port: Option<u16>,
    /// 秘密鍵ファイルのパス
    pub identity_file: Option<PathBuf>,
    /// その他のSSHオプション
    pub extra_options: Vec<String>,
}

impl Default for SshConnectionParams {
    fn default() -> Self {
        Self {
            port: None,
            identity_file: None,
            extra_options: Vec::new(),
        }
    }
}

/// SSH コマンド文字列を解析
///
/// # 例
/// ```
/// let params = parse_ssh_command("ssh -p 2222 -i ~/.ssh/mykey");
/// assert_eq!(params.port, Some(2222));
/// ```
pub fn parse_ssh_command(command: &str) -> SshConnectionParams {
    let mut params = SshConnectionParams::default();

    // コマンドを空白で分割
    let parts: Vec<&str> = command.split_whitespace().collect();

    let mut i = 0;
    while i < parts.len() {
        let part = parts[i];

        match part {
            "ssh" => {
                // sshコマンド自体はスキップ
            }
            "-p" | "--port" => {
                // ポート番号
                if i + 1 < parts.len() {
                    if let Ok(port) = parts[i + 1].parse::<u16>() {
                        params.port = Some(port);
                        i += 1; // 次の引数をスキップ
                    }
                }
            }
            "-i" | "--identity" => {
                // 秘密鍵ファイル
                if i + 1 < parts.len() {
                    let path = parts[i + 1];
                    // ~を展開
                    let expanded_path = if path.starts_with("~/") {
                        if let Some(home) = dirs::home_dir() {
                            home.join(&path[2..])
                        } else {
                            PathBuf::from(path)
                        }
                    } else {
                        PathBuf::from(path)
                    };
                    params.identity_file = Some(expanded_path);
                    i += 1;
                }
            }
            "-o" => {
                // SSHオプション
                if i + 1 < parts.len() {
                    params.extra_options.push(parts[i + 1].to_string());
                    i += 1;
                }
            }
            _ => {
                // その他のオプション
                if part.starts_with('-') {
                    params.extra_options.push(part.to_string());
                }
            }
        }

        i += 1;
    }

    params
}

/// ポート番号をコマンドから抽出（簡易版）
pub fn extract_port(command: &str) -> Option<u16> {
    parse_ssh_command(command).port
}

/// 秘密鍵ファイルをコマンドから抽出（簡易版）
pub fn extract_identity_file(command: &str) -> Option<PathBuf> {
    parse_ssh_command(command).identity_file
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_ssh_port() {
        let params = parse_ssh_command("ssh -p 2222");
        assert_eq!(params.port, Some(2222));
    }

    #[test]
    fn test_parse_ssh_identity() {
        let params = parse_ssh_command("ssh -i /path/to/key");
        assert_eq!(params.identity_file, Some(PathBuf::from("/path/to/key")));
    }

    #[test]
    fn test_parse_ssh_combined() {
        let params = parse_ssh_command("ssh -p 2222 -i ~/.ssh/mykey");
        assert_eq!(params.port, Some(2222));
        assert!(params.identity_file.is_some());
    }

    #[test]
    fn test_parse_ssh_complex() {
        let params = parse_ssh_command("ssh -p 22 -i ~/.ssh/id_rsa -o StrictHostKeyChecking=no");
        assert_eq!(params.port, Some(22));
        assert!(params.identity_file.is_some());
        assert_eq!(params.extra_options.len(), 1);
    }

    #[test]
    fn test_extract_port() {
        assert_eq!(extract_port("ssh -p 2222"), Some(2222));
        assert_eq!(extract_port("ssh"), None);
    }

    #[test]
    fn test_extract_identity_file() {
        let result = extract_identity_file("ssh -i /path/to/key");
        assert_eq!(result, Some(PathBuf::from("/path/to/key")));
        assert_eq!(extract_identity_file("ssh"), None);
    }
}
