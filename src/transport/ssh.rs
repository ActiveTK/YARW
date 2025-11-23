use std::net::TcpStream;
use std::path::PathBuf;
use ssh2::{Channel, Session};
use crate::error::{RsyncError, Result};
use std::io::Write;
use std::process::Command;
use tempfile::NamedTempFile;


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

fn generate_temp_public_key(private_key_path: &PathBuf) -> Result<NamedTempFile> {
    let output = Command::new("ssh-keygen")
        .arg("-y")
        .arg("-f")
        .arg(private_key_path)
        .output()
        .map_err(|e| RsyncError::Auth(format!(
            "Failed to execute ssh-keygen to extract public key: {}. \
            Please ensure ssh-keygen is installed and in PATH.",
            e
        )))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(RsyncError::Auth(format!(
            "Failed to extract public key from private key: {}",
            stderr
        )));
    }

    let mut temp_file = NamedTempFile::new()
        .map_err(|e| RsyncError::Io(e))?;

    temp_file.write_all(&output.stdout)
        .map_err(|e| RsyncError::Io(e))?;

    temp_file.flush()
        .map_err(|e| RsyncError::Io(e))?;

    Ok(temp_file)
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
                if !private_key_path.exists() {
                    return Err(RsyncError::Auth(format!(
                        "Private key file does not exist: {}",
                        private_key_path.display()
                    )));
                }

                if !private_key_path.is_file() {
                    return Err(RsyncError::Auth(format!(
                        "Private key path is not a file: {}",
                        private_key_path.display()
                    )));
                }

                let temp_public_key = generate_temp_public_key(&private_key_path)?;
                let public_key_path = temp_public_key.path();

                session.userauth_pubkey_file(
                    username,
                    Some(public_key_path),
                    &private_key_path,
                    None,
                ).map_err(|e| {
                    let msg = e.to_string();
                    if msg.contains("unknown error") || msg.contains("Session(-1)") {
                        RsyncError::Auth(format!(
                            "Public key authentication failed. Possible reasons:\n\
                            - The key file may require a passphrase (not currently supported)\n\
                            - The key format may be incompatible (try converting to OpenSSH format)\n\
                            - The key file may be corrupted\n\
                            - SSH2 error: {}",
                            msg
                        ))
                    } else {
                        RsyncError::Auth(format!("Public key authentication failed: {}", msg))
                    }
                })?;
            }
            AuthMethod::Password(password) => {
                session.userauth_password(username, &password)
                    .map_err(|e| RsyncError::Auth(e.to_string()))?;
            }
            AuthMethod::Agent => {
                let mut agent = session.agent()?;
                agent.connect().map_err(|e| {
                    let msg = e.to_string();
                    if msg.contains("Session(-42)") || msg.contains("unable to connect to agent pipe") {
                        RsyncError::Auth(
                            "SSH Agent is not running or not accessible. \
                            On Windows, ensure ssh-agent service is running or use public key authentication instead.".to_string()
                        )
                    } else {
                        RsyncError::Auth(format!("Failed to connect to SSH agent: {}", msg))
                    }
                })?;
                agent.list_identities().map_err(|e| {
                    RsyncError::Auth(format!("Failed to list SSH agent identities: {}", e))
                })?;

                let identities = agent.identities().map_err(|e| {
                    RsyncError::Auth(format!("Failed to get SSH agent identities: {}", e))
                })?;
                let identity = identities.get(0).ok_or_else(|| {
                    RsyncError::Auth("No SSH keys found in agent. Add a key with 'ssh-add' or use public key authentication.".to_string())
                })?;

                agent.userauth(username, identity)
                     .map_err(|e| RsyncError::Auth(format!("SSH agent authentication failed: {}", e)))?;
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
