use std::net::TcpStream;
use std::path::PathBuf;
use ssh2::{Channel, Session};
use crate::error::{RsyncError, Result};

/// SSH認証方式
#[allow(dead_code)]
pub enum AuthMethod {
    /// 公開鍵認証（秘密鍵のパス）
    PublicKey(PathBuf),
    /// パスワード認証
    Password(String),
    /// SSHエージェント認証
    Agent,
}

/// SSHトランスポート
pub struct SshTransport {
    session: Session,
}

impl SshTransport {
    /// SSHサーバーに接続し、認証を行う
    pub fn connect(
        host: &str,
        port: u16,
        username: &str,
        auth_method: AuthMethod,
    ) -> Result<Self> {
        let tcp = TcpStream::connect((host, port)).map_err(|e| RsyncError::Network(e.to_string()))?;
        let mut session = Session::new()?;
        session.set_tcp_stream(tcp);
        session.handshake().map_err(|e| RsyncError::Network(e.to_string()))?;

        match auth_method {
            AuthMethod::PublicKey(private_key_path) => {
                session.userauth_pubkey_file(
                    username,
                    None, // public key path (optional)
                    &private_key_path,
                    None, // passphrase (optional)
                ).map_err(|e| RsyncError::Auth(e.to_string()))?;
            }
            AuthMethod::Password(password) => {
                session.userauth_password(username, &password)
                    .map_err(|e| RsyncError::Auth(e.to_string()))?;
            }
            AuthMethod::Agent => {
                let mut agent = session.agent()?;
                agent.connect().map_err(|e| RsyncError::Auth(e.to_string()))?;
                agent.list_identities().map_err(|e| RsyncError::Auth(e.to_string()))?;
                
                let identities = agent.identities().map_err(|e| RsyncError::Auth(e.to_string()))?;
                let identity = identities.get(0).ok_or_else(|| RsyncError::Auth("No identities found in agent".to_string()))?;
                
                agent.userauth(username, identity)
                     .map_err(|e| RsyncError::Auth(e.to_string()))?;
            }
        }

        if !session.authenticated() {
            return Err(RsyncError::Auth("SSH authentication failed".to_string()));
        }

        Ok(SshTransport { session })
    }

    /// リモートでコマンドを実行し、その標準入出力を返す
    pub fn execute(&mut self, command: &str) -> Result<Channel> {
        let mut channel = self.session.channel_session().map_err(|e| RsyncError::RemoteExec(e.to_string()))?;
        channel.exec(command).map_err(|e| RsyncError::RemoteExec(e.to_string()))?;
        Ok(channel)
    }
}

