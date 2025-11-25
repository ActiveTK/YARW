use std::path::PathBuf;
use crate::error::{RsyncError, Result};
use std::io::Write;
use std::sync::Arc;
use russh::*;
use russh_keys::*;
use std::collections::VecDeque;

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

        Ok(SshChannel {
            channel,
            read_buffer: VecDeque::new(),
            write_seq: std::cell::Cell::new(0),
        })
    }
}

pub struct SshChannel {
    channel: russh::Channel<russh::client::Msg>,
    read_buffer: VecDeque<u8>,
    write_seq: std::cell::Cell<u32>,
}

impl std::io::Read for SshChannel {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        if buf.is_empty() {
            return Ok(0);
        }

        let handle = tokio::runtime::Handle::try_current()
            .expect("must be called from within a tokio runtime");

        tokio::task::block_in_place(|| {
            handle.block_on(async {
            while self.read_buffer.is_empty() {
                match self.channel.wait().await {
                    Some(ChannelMsg::Data { ref data }) => {
                        self.read_buffer.extend(data.iter().copied());
                    }
                    Some(ChannelMsg::Eof) => {
                        return Ok(0);
                    }
                    Some(ChannelMsg::ExitStatus { exit_status: _ }) => {
                        if self.read_buffer.is_empty() {
                            return Ok(0);
                        }
                        break;
                    }
                    Some(ChannelMsg::ExtendedData { ref data, ext: _ }) => {
                        let stderr_msg = String::from_utf8_lossy(data);
                        eprintln!("[SSH STDERR] {}", stderr_msg);
                        continue;
                    }
                    Some(_) => {
                        continue;
                    }
                    None => {
                        if self.read_buffer.is_empty() {
                            return Ok(0);
                        }
                        break;
                    }
                }
            }

            let len = buf.len().min(self.read_buffer.len());
            for i in 0..len {
                buf[i] = self.read_buffer.pop_front().unwrap();
            }
            Ok(len)
            })
        })
    }
}

impl std::io::Write for SshChannel {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let seq = self.write_seq.get();
        self.write_seq.set(seq + 1);
        eprintln!("[SSH #{:03}] Writing {} bytes: {:02x?}", seq, buf.len(), &buf[..buf.len().min(16)]);
        let handle = tokio::runtime::Handle::try_current()
            .expect("must be called from within a tokio runtime");

        tokio::task::block_in_place(|| {
            handle.block_on(async {
                self.channel
                    .data(buf)
                    .await
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
                Ok(buf.len())
            })
        })
    }

    fn flush(&mut self) -> std::io::Result<()> {
        eprintln!("[SSH] Flush called");
        Ok(())
    }
}

impl SshChannel {
    pub fn close(&mut self) -> Result<()> {
        let handle = tokio::runtime::Handle::try_current()
            .expect("must be called from within a tokio runtime");

        tokio::task::block_in_place(|| {
            handle.block_on(async {
                self.channel
                    .eof()
                    .await
                    .map_err(|e| RsyncError::Network(e.to_string()))?;
                Ok(())
            })
        })
    }

    pub fn wait_close(&mut self) -> Result<()> {
        let handle = tokio::runtime::Handle::try_current()
            .expect("must be called from within a tokio runtime");

        tokio::task::block_in_place(|| {
            handle.block_on(async {
                while let Some(_msg) = self.channel.wait().await {
                }
                Ok(())
            })
        })
    }

    pub fn stderr(&mut self) -> SshChannelStderr {
        SshChannelStderr {
            channel: &mut self.channel,
            stderr_buffer: VecDeque::new(),
        }
    }
}

pub struct SshChannelStderr<'a> {
    channel: &'a mut russh::Channel<russh::client::Msg>,
    stderr_buffer: VecDeque<u8>,
}

impl<'a> std::io::Read for SshChannelStderr<'a> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        if buf.is_empty() {
            return Ok(0);
        }

        let handle = tokio::runtime::Handle::try_current()
            .expect("must be called from within a tokio runtime");

        tokio::task::block_in_place(|| {
            handle.block_on(async {
            while self.stderr_buffer.is_empty() {
                match self.channel.wait().await {
                    Some(ChannelMsg::ExtendedData { ref data, ext: 1 }) => {
                        self.stderr_buffer.extend(data.iter().copied());
                    }
                    Some(ChannelMsg::Eof) | None => {
                        return Ok(0);
                    }
                    Some(_) => continue,
                }
            }

            let len = buf.len().min(self.stderr_buffer.len());
            for i in 0..len {
                buf[i] = self.stderr_buffer.pop_front().unwrap();
            }
            Ok(len)
            })
        })
    }
}
