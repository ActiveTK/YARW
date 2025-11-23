




use std::path::PathBuf;


#[derive(Debug, Clone)]
pub struct SshConnectionParams {

    pub port: Option<u16>,

    pub identity_file: Option<PathBuf>,

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








pub fn parse_ssh_command(command: &str) -> SshConnectionParams {
    let mut params = SshConnectionParams::default();

    let parts = tokenize_command(command);

    let mut i = 0;
    while i < parts.len() {
        let part = &parts[i];

        match part.as_str() {
            "ssh" => {

            }
            "-p" | "--port" => {

                if i + 1 < parts.len() {
                    if let Ok(port) = parts[i + 1].parse::<u16>() {
                        params.port = Some(port);
                        i += 1;
                    }
                }
            }
            "-i" | "--identity" => {

                if i + 1 < parts.len() {
                    let path = &parts[i + 1];
                    let path = path.trim_matches('"');

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

                if i + 1 < parts.len() {
                    params.extra_options.push(parts[i + 1].clone());
                    i += 1;
                }
            }
            _ => {

                if part.starts_with('-') {
                    params.extra_options.push(part.clone());
                }
            }
        }

        i += 1;
    }

    params
}

fn tokenize_command(command: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current_token = String::new();
    let mut in_quotes = false;
    let mut chars = command.chars().peekable();

    while let Some(ch) = chars.next() {
        match ch {
            '\\' if chars.peek() == Some(&'"') => {
                chars.next();
                current_token.push('"');
            }
            '\\' => {
                current_token.push('\\');
            }
            '"' => {
                in_quotes = !in_quotes;
            }
            ' ' | '\t' if !in_quotes => {
                if !current_token.is_empty() {
                    tokens.push(current_token.clone());
                    current_token.clear();
                }
            }
            _ => {
                current_token.push(ch);
            }
        }
    }

    if !current_token.is_empty() {
        tokens.push(current_token);
    }

    tokens
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
    fn test_parse_ssh_quoted_path() {
        let params = parse_ssh_command(r#"ssh -p 10022 -i "C:\Users\Test User\key.pem""#);
        assert_eq!(params.port, Some(10022));
        assert_eq!(params.identity_file, Some(PathBuf::from(r"C:\Users\Test User\key.pem")));
    }

    #[test]
    fn test_parse_ssh_escaped_quotes() {
        let params = parse_ssh_command(r#"ssh -i \"C:\Program Files\ssh\key\""#);
        assert!(params.identity_file.is_some());
    }
}
