use std::net::TcpStream;
use std::path::PathBuf;
use ssh2::{Channel, Session};
use crate::error::{RsyncError, Result};
use std::io::Write;


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


pub struct SshTransport {
    session: Session,
}

impl SshTransport {

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
                    None,
                    &private_key_path,
                    None,
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


    pub fn execute(&mut self, command: &str) -> Result<Channel> {
        let mut channel = self.session.channel_session().map_err(|e| RsyncError::RemoteExec(e.to_string()))?;
        channel.exec(command).map_err(|e| RsyncError::RemoteExec(e.to_string()))?;
        Ok(channel)
    }
}
