use std::net::TcpStream;
use std::path::PathBuf;
use ssh2::{Channel, Session};
use crate::error::{RsyncError, Result};
use std::io::Write;
use std::fs;
use tempfile::NamedTempFile;
use rsa::pkcs1::DecodeRsaPrivateKey;
use rsa::traits::PublicKeyParts;


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

fn extract_public_key_from_private(private_key_path: &PathBuf) -> Result<NamedTempFile> {
    let key_data_str = fs::read_to_string(private_key_path)
        .map_err(|e| RsyncError::Auth(format!("Failed to read private key file: {}", e)))?;

    let public_key_str = if key_data_str.contains("BEGIN RSA PRIVATE KEY") {
        let rsa_key = rsa::RsaPrivateKey::from_pkcs1_pem(&key_data_str)
            .map_err(|e| RsyncError::Auth(format!("Failed to parse PKCS#1 RSA private key: {}", e)))?;

        let public_key = rsa::RsaPublicKey::from(&rsa_key);

        let mut e = public_key.e().to_bytes_be();
        let mut n = public_key.n().to_bytes_be();

        if e.first().map(|&b| b & 0x80 != 0).unwrap_or(false) {
            e.insert(0, 0x00);
        }
        if n.first().map(|&b| b & 0x80 != 0).unwrap_or(false) {
            n.insert(0, 0x00);
        }

        let mut ssh_pubkey_bytes = Vec::new();

        let key_type = b"ssh-rsa";
        ssh_pubkey_bytes.extend_from_slice(&(key_type.len() as u32).to_be_bytes());
        ssh_pubkey_bytes.extend_from_slice(key_type);

        ssh_pubkey_bytes.extend_from_slice(&(e.len() as u32).to_be_bytes());
        ssh_pubkey_bytes.extend_from_slice(&e);

        ssh_pubkey_bytes.extend_from_slice(&(n.len() as u32).to_be_bytes());
        ssh_pubkey_bytes.extend_from_slice(&n);

        use base64::Engine;
        let public_key_b64 = base64::engine::general_purpose::STANDARD.encode(&ssh_pubkey_bytes);

        format!("ssh-rsa {}\n", public_key_b64)
    } else {
        let key_data = fs::read(private_key_path)
            .map_err(|e| RsyncError::Auth(format!("Failed to read private key file: {}", e)))?;

        let private_key = ssh_key::PrivateKey::from_bytes(&key_data)
            .map_err(|e| RsyncError::Auth(format!("Failed to parse private key: {}", e)))?;

        let public_key = private_key.public_key();
        public_key.to_openssh()
            .map_err(|e| RsyncError::Auth(format!("Failed to serialize public key: {}", e)))?
    };

    let mut temp_file = NamedTempFile::new()
        .map_err(|e| RsyncError::Io(e))?;

    temp_file.write_all(public_key_str.as_bytes())
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

                let mut public_key_path = private_key_path.clone();
                public_key_path.set_extension("pub");

                let alt_pub_path = private_key_path.with_file_name(
                    format!("{}.pub", private_key_path.file_name().unwrap().to_string_lossy())
                );

                let result = if public_key_path.exists() {
                    session.userauth_pubkey_file(
                        username,
                        Some(&public_key_path),
                        &private_key_path,
                        None,
                    )
                } else if alt_pub_path.exists() {
                    session.userauth_pubkey_file(
                        username,
                        Some(&alt_pub_path),
                        &private_key_path,
                        None,
                    )
                } else {
                    let private_key_str = fs::read_to_string(&private_key_path)
                        .map_err(|e| RsyncError::Auth(format!("Failed to read private key: {}", e)))?;

                    if private_key_str.contains("BEGIN RSA PRIVATE KEY") {
                        let rsa_key = rsa::RsaPrivateKey::from_pkcs1_pem(&private_key_str)
                            .map_err(|e| RsyncError::Auth(format!("Failed to parse private key: {}", e)))?;

                        use rsa::pkcs8::EncodePrivateKey;
                        let openssh_key_pem = rsa_key.to_pkcs8_pem(rsa::pkcs8::LineEnding::LF)
                            .map_err(|e| RsyncError::Auth(format!("Failed to convert key to PKCS#8: {}", e)))?;

                        let mut temp_private_key = NamedTempFile::new()
                            .map_err(|e| RsyncError::Io(e))?;
                        temp_private_key.write_all(openssh_key_pem.as_bytes())
                            .map_err(|e| RsyncError::Io(e))?;
                        temp_private_key.flush()
                            .map_err(|e| RsyncError::Io(e))?;

                        let temp_pub_key = extract_public_key_from_private(&private_key_path)?;

                        let result = session.userauth_pubkey_file(
                            username,
                            Some(temp_pub_key.path()),
                            temp_private_key.path(),
                            None,
                        );

                        result
                    } else {
                        let temp_pub_key = extract_public_key_from_private(&private_key_path)?;
                        session.userauth_pubkey_file(
                            username,
                            Some(temp_pub_key.path()),
                            &private_key_path,
                            None,
                        )
                    }
                };

                result.map_err(|e| {
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
