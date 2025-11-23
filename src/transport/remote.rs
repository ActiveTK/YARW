use crate::options::Options;
use crate::error::{Result, RsyncError};
use super::{SshTransport, AuthMethod, SyncStats, prompt_for_password};
use super::ssh_command::parse_ssh_command;
use crate::filesystem::{path_utils::{is_remote_path, parse_remote_path, to_unix_separators}, Scanner};
use crate::protocol::{ProtocolStream, FileList, PROTOCOL_VERSION_MAX};
use std::path::{Path, PathBuf};
use std::io::Read;
use std::fs;
use std::time::Instant;

pub struct RemoteTransport {
    options: Options,
}


impl RemoteTransport {
    pub fn new(options: Options) -> Self {
        Self { options }
    }

    pub fn sync(&self, source: &str, destination: &str) -> Result<SyncStats> {
        let start_time = Instant::now();
        let mut stats = SyncStats::default();
        let is_remote_source = is_remote_path(source);
        let (user_host, remote_raw_path) = if is_remote_source {
            parse_remote_path(source)
        } else {
            parse_remote_path(destination)
        };

        let local_path_str = if is_remote_source {
            destination
        } else {
            source
        };
        let local_path = Path::new(local_path_str);

        if let Some((user, host)) = user_host {
            let username = if user.is_empty() {
                whoami::username()
            } else {
                user
            };

            let port = if let Some(ref rsh_command) = self.options.rsh {
                let params = parse_ssh_command(rsh_command);
                params.port.unwrap_or(22)
            } else {
                22
            };

            let verbose = self.options.verbose_output();
            verbose.print_verbose(&format!("Connecting to {}@{}:{} ...", username, host, port));

            let mut transport_result: Option<SshTransport> = None;
            let mut last_error: Option<String> = None;

            let handle = tokio::runtime::Handle::try_current()
                .map_err(|e| RsyncError::Network(format!("Not running in tokio runtime: {}", e)))?;

            if let Some(ref rsh_command) = self.options.rsh {
                let params = parse_ssh_command(rsh_command);
                if let Some(identity_file) = params.identity_file {
                    verbose.print_verbose(&format!("Trying public key authentication: {}", identity_file.display()));
                    match tokio::task::block_in_place(|| handle.block_on(SshTransport::connect(&host, port, &username, AuthMethod::PublicKey(identity_file.clone())))) {
                        Ok(transport) => {
                            verbose.print_verbose("Public key authentication successful.");
                            transport_result = Some(transport);
                        }
                        Err(e) => {
                            verbose.print_verbose(&format!("Public key authentication failed: {}", e));
                            last_error = Some(e.to_string());
                        }
                    }
                }
            }

            if transport_result.is_none() {
                verbose.print_verbose("Trying SSH agent authentication...");
                match tokio::task::block_in_place(|| handle.block_on(SshTransport::connect(&host, port, &username, AuthMethod::Agent))) {
                    Ok(transport) => {
                        verbose.print_verbose("SSH agent authentication successful.");
                        transport_result = Some(transport);
                    }
                    Err(e) => {
                        verbose.print_verbose(&format!("SSH agent authentication failed: {}", e));
                        last_error = Some(e.to_string());
                    }
                }
            }

            if transport_result.is_none() {
                verbose.print_verbose("Trying password authentication...");
                match prompt_for_password(&username, &host) {
                    Ok(password) => {
                        match tokio::task::block_in_place(|| handle.block_on(SshTransport::connect(&host, port, &username, AuthMethod::Password(password)))) {
                            Ok(transport) => {
                                verbose.print_verbose("Password authentication successful.");
                                transport_result = Some(transport);
                            }
                            Err(e) => {
                                verbose.print_error(&format!("Password authentication failed: {}", e));
                                last_error = Some(e.to_string());
                            }
                        }
                    }
                    Err(e) => {
                        verbose.print_error(&format!("Failed to read password: {}", e));
                        last_error = Some(e.to_string());
                    }
                }
            }

            match transport_result {
                Some(mut transport) => {
                    verbose.print_verbose("SSH connection successful.");


                    let remote_unix_path = to_unix_separators(&remote_raw_path);


                    let mut rsync_args = vec![
                        "--server",
                        "--sender",
                    ];


                    if self.options.recursive { rsync_args.push("-r"); }
                    if self.options.verbose > 0 { rsync_args.push("-v"); }
                    if self.options.delete { rsync_args.push("--delete"); }

                    rsync_args.push(".");
                    rsync_args.push(&remote_unix_path);

                    let rsync_command_str = format!("rsync {}", rsync_args.join(" "));
                    verbose.print_debug(&format!("Executing remote command: {}", rsync_command_str));

                    match tokio::task::block_in_place(|| handle.block_on(transport.execute(&rsync_command_str))) {
                        Ok(mut channel) => {

                            let mut stream = ProtocolStream::new(&mut channel, PROTOCOL_VERSION_MAX);




                            verbose.print_verbose("Negotiating protocol version...");
                            stream.write_i32(PROTOCOL_VERSION_MAX)?;
                            stream.flush()?;
                            let remote_version = stream.read_i32()?;


                            stream.write_i32(PROTOCOL_VERSION_MAX)?;
                            stream.flush()?;
                            let _remote_version_ack = stream.read_i32()?;

                            verbose.print_verbose(&format!("Negotiated protocol version: {}", remote_version));


                            let scanner = Scanner::new()
                                .recursive(self.options.recursive)
                                .follow_symlinks(self.options.copy_links);
                            let local_file_infos = scanner.scan(local_path)?;


                            verbose.print_verbose("Sending file list...");
                            FileList::encode(&mut stream, &local_file_infos)?;
                            verbose.print_verbose("File list sent.");


                            verbose.print_verbose("Receiving remote file list...");
                            let remote_file_infos = FileList::decode(&mut stream)?;
                            verbose.print_verbose(&format!("Received {} remote files.", remote_file_infos.len()));
                            stats.scanned_files = local_file_infos.len();


                            verbose.print_verbose("Starting file transfer...");


                            for local_file in &local_file_infos {
                                if local_file.is_directory() {

                                    continue;
                                }


                                let remote_file = remote_file_infos.iter()
                                    .find(|f| f.path == local_file.path);

                                verbose.print_basic(&format!("Processing: {}", local_file.path.display()));



                                if remote_file.is_some() {
                                    verbose.print_verbose("  Updating existing file (whole-file transfer)");
                                } else {
                                    verbose.print_verbose("  New file");
                                }


                                let local_file_path = local_path.join(&local_file.path);
                                if local_file_path.exists() {
                                    let file_data = fs::read(&local_file_path)?;


                                    stream.write_varint(file_data.len() as i64)?;


                                    stream.write_all(&file_data)?;
                                    stream.flush()?;

                                    stats.transferred_files += 1;
                                    stats.transferred_bytes += file_data.len() as u64;

                                    verbose.print_basic(&format!("  Transferred {} bytes", file_data.len()));
                                }
                            }


                            stats.execution_time_secs = start_time.elapsed().as_secs_f64();

                            verbose.print_basic("Transfer complete!");
                            if self.options.stats {
                                stats.display(self.options.human_readable, &verbose);
                            }


                            let mut stderr_bytes = Vec::new();
                            match channel.stderr().read_to_end(&mut stderr_bytes) {
                                Ok(_) => {
                                    if !stderr_bytes.is_empty() {
                                        verbose.print_error(&format!("Remote stderr: {}", String::from_utf8_lossy(&stderr_bytes)));
                                    }
                                },
                                Err(e) => verbose.print_error(&format!("Failed to read remote stderr: {}", e)),
                            }


                            channel.close()?;
                            channel.wait_close()?;

                        }
                        Err(e) => return Err(RsyncError::RemoteExec(format!("Failed to execute remote command: {}", e))),
                    }
                }
                None => {
                    let error_msg = last_error.unwrap_or_else(|| "All authentication methods failed".to_string());
                    return Err(RsyncError::Auth(format!("SSH connection failed: {}", error_msg)));
                }
            }
        } else {
            return Err(RsyncError::InvalidPath(PathBuf::from(source)));
        }

        Ok(stats)
    }
}
