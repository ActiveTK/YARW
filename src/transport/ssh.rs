use std::path::PathBuf;
use crate::error::{RsyncError, Result};
use std::io::Write;
use std::sync::Arc;
use russh::*;
use russh_keys::*;
use tokio::io::{AsyncRead, AsyncWrite};
use std::pin::Pin;

pub enum AuthMethod {
    PublicKey(PathBuf),
    Password(String),
    Agent,
}

pub fn prompt_for_password(username: &str, host: &str) -> Result<String> {
    print!("{}@{}'s password: ", username, host);
    std::io::stdout().flush().map_err(|e| RsyncError::Io(e))?;

    let password = rpassword::read_password()
        .map_err(|e| RsyncError::Auth(format!("Failed to read password: {}", e)))?;

    if password.is_empty() {
        return Err(RsyncError::Auth("Password cannot be empty".to_string()));
    }

    Ok(password)
}

struct Client;

#[async_trait::async_trait]
impl client::Handler for Client {
    type Error = russh::Error;

    async fn check_server_key(
        &mut self,
        _server_public_key: &key::PublicKey,
    ) -> std::result::Result<bool, Self::Error> {
        Ok(true)
    }
}

pub struct SshTransport {
    session: client::Handle<Client>,
}

impl SshTransport {
    pub async fn connect(
        host: &str,
        port: u16,
        username: &str,
        auth_method: AuthMethod,
    ) -> Result<Self> {
        let config = client::Config::default();
        let sh = Client;

        let mut session = client::connect(Arc::new(config), (host, port), sh)
            .await
            .map_err(|e| RsyncError::Network(e.to_string()))?;

        match auth_method {
            AuthMethod::PublicKey(private_key_path) => {
                if !private_key_path.exists() {
                    return Err(RsyncError::Auth(format!(
                        "Private key file does not exist: {}",
                        private_key_path.display()
                    )));
                }

                let key_pair = load_secret_key(&private_key_path, None)
                    .map_err(|e| RsyncError::Auth(format!("Failed to load private key: {}", e)))?;

                let auth_res = session
                    .authenticate_publickey(username, Arc::new(key_pair))
                    .await
                    .map_err(|e| RsyncError::Auth(format!("Public key authentication failed: {}", e)))?;

                if !auth_res {
                    return Err(RsyncError::Auth("Public key authentication rejected by server".to_string()));
                }
            }
            AuthMethod::Password(password) => {
                let auth_res = session
                    .authenticate_password(username, &password)
                    .await
                    .map_err(|e| RsyncError::Auth(format!("Password authentication failed: {}", e)))?;

                if !auth_res {
                    return Err(RsyncError::Auth("Password authentication rejected by server".to_string()));
                }
            }
            AuthMethod::Agent => {
                return Err(RsyncError::Auth(
                    "SSH Agent authentication not yet implemented with russh".to_string()
                ));
            }
        }

        Ok(SshTransport { session })
    }

    pub async fn execute(&mut self, command: &str) -> Result<SshChannel> {
        let channel = self.session
            .channel_open_session()
            .await
            .map_err(|e| RsyncError::RemoteExec(format!("Failed to open channel: {}", e)))?;

        channel
            .exec(true, command)
            .await
            .map_err(|e| RsyncError::RemoteExec(format!("Failed to execute command: {}", e)))?;

        Ok(SshChannel { channel })
    }
}

pub struct SshChannel {
    channel: russh::Channel<russh::client::Msg>,
}

impl std::io::Read for SshChannel {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let rt = tokio::runtime::Handle::try_current()
            .or_else(|_| {
                tokio::runtime::Runtime::new().map(|rt| rt.handle().clone())
            })
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

        rt.block_on(async {
            use tokio::io::AsyncReadExt;
            self.channel.read(buf).await
        })
    }
}

impl std::io::Write for SshChannel {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let rt = tokio::runtime::Handle::try_current()
            .or_else(|_| {
                tokio::runtime::Runtime::new().map(|rt| rt.handle().clone())
            })
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

        rt.block_on(async {
            use tokio::io::AsyncWriteExt;
            self.channel.write(buf).await
        })
    }

    fn flush(&mut self) -> std::io::Result<()> {
        let rt = tokio::runtime::Handle::try_current()
            .or_else(|_| {
                tokio::runtime::Runtime::new().map(|rt| rt.handle().clone())
            })
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

        rt.block_on(async {
            use tokio::io::AsyncWriteExt;
            self.channel.flush().await
        })
    }
}

impl SshChannel {
    pub fn close(&mut self) -> Result<()> {
        let rt = tokio::runtime::Handle::try_current()
            .or_else(|_| {
                tokio::runtime::Runtime::new().map(|rt| rt.handle().clone())
            })
            .map_err(|e| RsyncError::Network(e.to_string()))?;

        rt.block_on(async {
            self.channel
                .eof()
                .await
                .map_err(|e| RsyncError::Network(e.to_string()))?;
            Ok(())
        })
    }

    pub fn wait_close(&mut self) -> Result<()> {
        let rt = tokio::runtime::Handle::try_current()
            .or_else(|_| {
                tokio::runtime::Runtime::new().map(|rt| rt.handle().clone())
            })
            .map_err(|e| RsyncError::Network(e.to_string()))?;

        rt.block_on(async {
            self.channel
                .wait()
                .await
                .map_err(|e| RsyncError::Network(e.to_string()))?;
            Ok(())
        })
    }

    pub fn stderr(&mut self) -> SshChannelStderr {
        SshChannelStderr {
            channel: &mut self.channel,
        }
    }
}

pub struct SshChannelStderr<'a> {
    channel: &'a mut russh::Channel<russh::client::Msg>,
}

impl<'a> std::io::Read for SshChannelStderr<'a> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let rt = tokio::runtime::Handle::try_current()
            .or_else(|_| {
                tokio::runtime::Runtime::new().map(|rt| rt.handle().clone())
            })
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

        rt.block_on(async {
            use tokio::io::AsyncReadExt;
            match self.channel.wait().await {
                Ok(_) => Ok(0),
                Err(e) => Err(std::io::Error::new(std::io::ErrorKind::Other, e)),
            }
        })
    }
}
