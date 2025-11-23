use crate::transport::daemon_config::{DaemonConfig, ModuleConfig};
use crate::protocol::{AsyncProtocolStream, PROTOCOL_VERSION_MAX};
use crate::filesystem::Scanner;
use crate::output::VerboseOutput;
use tokio::net::{TcpListener, TcpStream};
use anyhow::{Result, Context, bail};
use std::fs;

pub struct RsyncDaemon {
    config: DaemonConfig,
}

impl RsyncDaemon {
    pub fn new(config: DaemonConfig) -> Self {
        RsyncDaemon { config }
    }

    pub async fn start(&self) -> Result<()> {
        let verbose = VerboseOutput::new(1, false);
        let addr = format!("{}:{}", self.config.address, self.config.port);
        let listener = TcpListener::bind(&addr).await.context(format!("Failed to bind to {}", addr))?;
        verbose.print_basic(&format!("Rsync daemon listening on {}", addr));

        loop {
            let (socket, peer_addr) = listener.accept().await?;
            verbose.print_basic(&format!("Client connected from: {}", peer_addr));
            let config_clone = self.config.clone();
            tokio::spawn(async move {
                let verbose = VerboseOutput::new(1, false);
                if let Err(e) = Self::handle_client(socket, &config_clone).await {
                    verbose.print_error(&format!("handling client {}: {}", peer_addr, e));
                }
            });
        }
    }

    async fn handle_client(socket: TcpStream, config: &DaemonConfig) -> Result<()> {
        let verbose = VerboseOutput::new(1, false);
        let mut stream = AsyncProtocolStream::new(socket, PROTOCOL_VERSION_MAX);


        verbose.print_verbose("Negotiating protocol version...");
        let client_version = stream.read_i32().await?;
        verbose.print_verbose(&format!("Client version: {}", client_version));


        stream.write_i32(PROTOCOL_VERSION_MAX).await?;
        stream.flush().await?;


        stream.write_i32(PROTOCOL_VERSION_MAX).await?;
        stream.flush().await?;
        let _client_version_ack = stream.read_i32().await?;


        verbose.print_verbose("Waiting for module name...");
        let module_name = stream.read_string(256).await?;
        verbose.print_verbose(&format!("Client requested module: {}", module_name));


        let module_config = config.modules.get(&module_name)
            .ok_or_else(|| anyhow::anyhow!("Module '{}' not found", module_name))?;


        if let Some(ref auth_users) = module_config.auth_users {
            verbose.print_verbose(&format!("Authentication required for module '{}'", module_name));
            if !Self::authenticate(&mut stream, auth_users, &module_config).await? {
                bail!("Authentication failed");
            }
            verbose.print_verbose("Authentication successful");
        }


        Self::handle_file_transfer(&mut stream, module_config).await?;

        verbose.print_basic("Client session completed successfully");
        Ok(())
    }

    async fn authenticate(
        stream: &mut AsyncProtocolStream<TcpStream>,
        _auth_users: &[String],
        module_config: &ModuleConfig,
    ) -> Result<bool> {
        let verbose = VerboseOutput::new(1, false);

        stream.write_string("@RSYNCD: AUTHREQD").await?;
        stream.flush().await?;


        let username = stream.read_string(256).await?;
        verbose.print_verbose(&format!("Authentication attempt for user: {}", username));


        let password_hash = stream.read_string(512).await?;


        if let Some(ref secrets_file) = module_config.secrets_file {
            if secrets_file.exists() {
                let contents = fs::read_to_string(secrets_file)?;
                for line in contents.lines() {
                    if line.trim().is_empty() || line.starts_with('#') {
                        continue;
                    }
                    let parts: Vec<&str> = line.splitn(2, ':').collect();
                    if parts.len() == 2 {
                        let (file_user, file_pass) = (parts[0].trim(), parts[1].trim());
                        if file_user == username {

                            if password_hash == file_pass {
                                return Ok(true);
                            }
                        }
                    }
                }
            }
        }


        stream.write_string("@RSYNCD: AUTH FAILED").await?;
        stream.flush().await?;
        Ok(false)
    }

    async fn handle_file_transfer(
        stream: &mut AsyncProtocolStream<TcpStream>,
        module_config: &ModuleConfig,
    ) -> Result<()> {
        let verbose = VerboseOutput::new(1, false);
        verbose.print_verbose(&format!("Starting file transfer for path: {:?}", module_config.path));


        let scanner = Scanner::new().recursive(true);
        let files = scanner.scan(&module_config.path)?;
        verbose.print_verbose(&format!("Scanned {} files", files.len()));


        stream.write_varint(files.len() as i64).await?;


        for file in &files {

            let relative_path = file.path.strip_prefix(&module_config.path)
                .unwrap_or(&file.path);
            stream.write_string(&relative_path.to_string_lossy()).await?;


            stream.write_varint(file.size as i64).await?;


            let mtime_secs = file.mtime.duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            stream.write_varint(mtime_secs as i64).await?;


            let file_type_code = if file.is_directory() { 1i8 } else { 0i8 };
            stream.write_i8(file_type_code).await?;
        }

        stream.flush().await?;
        verbose.print_verbose("File list sent");


        if !module_config.read_only {
            verbose.print_verbose("Receiving files from client...");

            let num_files = stream.read_varint().await? as usize;
            verbose.print_verbose(&format!("Client sending {} files", num_files));

            for i in 0..num_files {
                let file_path = stream.read_string(4096).await?;
                let file_size = stream.read_varint().await? as usize;

                verbose.print_verbose(&format!("Receiving file {}: {} ({} bytes)", i + 1, file_path, file_size));

                let dest_path = module_config.path.join(&file_path);


                if let Some(parent) = dest_path.parent() {
                    fs::create_dir_all(parent)?;
                }


                let mut file_data = vec![0u8; file_size];
                stream.read_all(&mut file_data).await?;
                fs::write(&dest_path, &file_data)?;

                verbose.print_verbose(&format!("Saved file: {:?}", dest_path));
            }
        }

        verbose.print_basic("File transfer completed");
        Ok(())
    }
}
