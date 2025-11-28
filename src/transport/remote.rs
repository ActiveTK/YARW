use crate::options::Options;
use crate::error::{Result, RsyncError};
use super::{SshTransport, AuthMethod, SyncStats, prompt_for_password};
use super::ssh_command::parse_ssh_command;
use crate::filesystem::{path_utils::{is_remote_path, parse_remote_path, to_unix_separators}, Scanner};
use crate::protocol::{PROTOCOL_VERSION_MAX, MultiplexIO};
use std::path::{Path, PathBuf};
use std::io::{Read, Write};
use std::fs;
use std::time::Instant;
use byteorder::WriteBytesExt;

pub struct RemoteTransport {
    options: Options,
}


impl RemoteTransport {
    pub fn new(options: Options) -> Self {
        Self { options }
    }

    fn handle_multiplexed_protocol<T: Read + Write>(
        mut channel: T,
        is_remote_source: bool,
        local_path: &Path,
        negotiated_version: i32,
        compat_flags: &crate::protocol::CompatFlags,
        options: &Options,
        verbose: &crate::output::verbose::VerboseOutput,
        stats: &mut SyncStats,
        start_time: Instant,
    ) -> Result<()> {
        use crate::protocol::{ExcludeList, send_file_list, recv_file_list, MultiplexWriter};
        use crate::filesystem::Scanner;

        let local_file_infos = if !is_remote_source {
            let scanner = Scanner::new()
                .recursive(options.recursive)
                .follow_symlinks(options.copy_links);
            let files = scanner.scan(local_path)?;

            verbose.print_verbose(&format!("Sending file list ({} files)...", files.len()));
            send_file_list(&mut channel, &files, local_path, negotiated_version, compat_flags)?;
            verbose.print_verbose("File list sent.");

            files
        } else {
            verbose.print_verbose("Skipping file list send (remote source mode)");
            Vec::new()
        };

        verbose.print_verbose("Receiving remote file list...");
        let remote_file_entries = recv_file_list(&mut channel, negotiated_version, compat_flags)?;
        verbose.print_verbose(&format!("Received {} remote files.", remote_file_entries.len()));
        stats.scanned_files = local_file_infos.len();

        verbose.print_verbose("Starting file transfer...");

        if is_remote_source {
            use crate::protocol::{read_ndx_and_attrs, NdxState, NDX_DONE, recv_id_lists, write_ndx, read_sum_head, read_int};

            verbose.print_verbose("Receiving UID/GID lists...");
            recv_id_lists(&mut channel)?;
            verbose.print_verbose("UID/GID lists received.");

            for remote_entry in &remote_file_entries {
                if remote_entry.is_dir {
                    let dir_path = local_path.join(&remote_entry.path);
                    if !dir_path.exists() {
                        verbose.print_verbose(&format!("Creating directory: {}", dir_path.display()));
                        fs::create_dir_all(&dir_path)?;
                    }
                }
            }

            verbose.print_verbose("Acting as generator: sending file requests...");
            let mut ndx_state_gen = NdxState::new();

            for (idx, remote_entry) in remote_file_entries.iter().enumerate() {
                if remote_entry.is_dir {
                    continue;
                }

                verbose.print_verbose(&format!("Requesting file {}: {}", idx, remote_entry.path.display()));

                write_ndx(&mut channel, idx as i32, &mut ndx_state_gen, negotiated_version)?;

                if negotiated_version >= 29 {
                    use crate::protocol::{write_shortint, ITEM_TRANSFER};
                    let iflags = ITEM_TRANSFER;
                    write_shortint(&mut channel, iflags)?;
                    verbose.print_verbose(&format!("  Sent iflags: {:#06x}", iflags));
                }

                channel.write_i32::<byteorder::LittleEndian>(0)?;
                channel.write_i32::<byteorder::LittleEndian>(0)?;
                if negotiated_version >= 27 {
                    channel.write_i32::<byteorder::LittleEndian>(0)?;
                }
                channel.write_i32::<byteorder::LittleEndian>(0)?;
            }

            verbose.print_verbose("Sending NDX_DONE to complete generator phase");
            write_ndx(&mut channel, NDX_DONE, &mut ndx_state_gen, negotiated_version)?;
            channel.flush()?;

            verbose.print_verbose("Acting as receiver: receiving file data...");
            let mut ndx_state_recv = NdxState::new();

            loop {
                let (file_ndx, iflags, _fnamecmp_type, _xname) = read_ndx_and_attrs(&mut channel, &mut ndx_state_recv, negotiated_version)?;
                if file_ndx == NDX_DONE {
                    verbose.print_verbose("Received NDX_DONE from sender");
                    break;
                }

                verbose.print_verbose(&format!("Received file index: {}, iflags: {:#06x}", file_ndx, iflags));

                if file_ndx < 0 || file_ndx >= remote_file_entries.len() as i32 {
                    return Err(RsyncError::Other(format!("Invalid file index from sender: {}", file_ndx)));
                }

                let remote_entry = &remote_file_entries[file_ndx as usize];
                verbose.print_basic(&format!("Receiving file {}: {}", file_ndx, remote_entry.path.display()));

                let file_path = local_path.join(&remote_entry.path);
                if let Some(parent) = file_path.parent() {
                    if !parent.exists() {
                        fs::create_dir_all(parent)?;
                    }
                }

                use std::io::Read;
                let mut file_data = Vec::new();

                let (sum_count, sum_blength, sum_s2length, sum_remainder) = read_sum_head(&mut channel, negotiated_version)?;
                verbose.print_verbose(&format!("  Sum header: count={}, blength={}, s2length={}, remainder={}", sum_count, sum_blength, sum_s2length, sum_remainder));

                let file_size = remote_entry.len;
                verbose.print_verbose(&format!("  Expected file size: {} bytes", file_size));

                let mut received = 0;
                loop {
                    let token = read_int(&mut channel)?;
                    verbose.print_verbose(&format!("    Token: {} (received so far: {})", token, received));

                    if token == 0 {
                        verbose.print_verbose("    End of file marker (token=0)");
                        break;
                    }

                    if token > 0 {
                        let len = token as usize;
                        verbose.print_verbose(&format!("    Reading {} bytes of literal data", len));
                        let mut chunk = vec![0u8; len];
                        channel.read_exact(&mut chunk)?;
                        verbose.print_verbose(&format!("    First 20 bytes: {:?}", &chunk[..chunk.len().min(20)]));
                        file_data.extend_from_slice(&chunk);
                        received += len;
                    } else {
                        verbose.print_verbose(&format!("    Block reference: {}", -token));
                    }
                }

                fs::write(&file_path, &file_data)?;

                stats.transferred_files += 1;
                stats.transferred_bytes += file_data.len() as u64;

                verbose.print_basic(&format!("  Received {} bytes", file_data.len()));
            }
        } else {
            verbose.print_verbose("Sending files to remote...");
            for local_file in &local_file_infos {
                if local_file.is_directory() {
                    continue;
                }

                verbose.print_basic(&format!("Sending: {}", local_file.path.display()));

                let local_file_path = local_path.join(&local_file.path);
                if local_file_path.exists() {
                    let file_data = fs::read(&local_file_path)?;

                    use crate::protocol::write_varlong30;
                    write_varlong30(&mut channel, file_data.len() as i64)?;

                    channel.write_all(&file_data)?;

                    stats.transferred_files += 1;
                    stats.transferred_bytes += file_data.len() as u64;

                    verbose.print_basic(&format!("  Sent {} bytes", file_data.len()));
                }
            }
        }

        stats.execution_time_secs = start_time.elapsed().as_secs_f64();

        verbose.print_basic("Transfer complete!");
        if options.stats {
            stats.display(options.human_readable, verbose);
        }

        Ok(())
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


                    let mut rsync_command = String::from("rsync --server");

                    if is_remote_source {
                        rsync_command.push_str(" --sender");
                    }

                    let mut option_string = String::new();

                    if self.options.verbose > 0 {
                        option_string.push('v');
                    }

                    if self.options.archive {
                        option_string.push_str("logDtpre");
                    } else {
                        if self.options.recursive { option_string.push('r'); }
                    }

                    if self.options.itemize_changes {
                        option_string.push('i');
                    }

                    if !option_string.is_empty() {
                        rsync_command.push_str(&format!(" -{}", option_string));
                    }

                    if self.options.delete {
                        rsync_command.push_str(" --delete");
                    }

                    rsync_command.push_str(" --no-inc-recursive");

                    rsync_command.push_str(" .");
                    rsync_command.push(' ');
                    rsync_command.push_str(&remote_unix_path);

                    let rsync_command_str = rsync_command;
                    verbose.print_debug(&format!("Executing remote command: {}", rsync_command_str));

                    match tokio::task::block_in_place(|| handle.block_on(transport.execute(&rsync_command_str))) {
                        Ok(mut channel) => {
                            use crate::protocol::{CompatFlags, send_file_list, recv_file_list, CF_VARINT_FLIST_FLAGS, ExcludeList, MultiplexIO};

                            verbose.print_verbose("Negotiating protocol version...");
                            let mut remote_version_bytes = [0u8; 4];
                            channel.read_exact(&mut remote_version_bytes)?;
                            let remote_version = i32::from_le_bytes(remote_version_bytes);

                            channel.write_all(&PROTOCOL_VERSION_MAX.to_le_bytes())?;

                            let negotiated_version = PROTOCOL_VERSION_MAX.min(remote_version);
                            verbose.print_verbose(&format!("Protocol versions: local={}, remote={}, negotiated={}", PROTOCOL_VERSION_MAX, remote_version, negotiated_version));

                            let (compat_flags, do_negotiated_strings) = if negotiated_version >= 30 {
                                verbose.print_verbose("Receiving compatibility flags from server...");
                                let remote_compat_flags = CompatFlags::read(&mut channel)?;
                                verbose.print_verbose(&format!("Server compat flags: 0x{:02x}", remote_compat_flags.flags));

                                let do_neg_strings = remote_compat_flags.has_flag(CF_VARINT_FLIST_FLAGS);
                                verbose.print_verbose(&format!("Negotiated strings: {}", do_neg_strings));

                                (remote_compat_flags, do_neg_strings)
                            } else {
                                (CompatFlags { flags: 0 }, false)
                            };

                            if negotiated_version >= 30 && do_negotiated_strings {
                                use crate::protocol::{write_vstring, read_vstring};

                                verbose.print_verbose("Negotiating algorithms...");

                                write_vstring(&mut channel, "md5 md4")?;
                                verbose.print_verbose("Sent checksum list: md5 md4");

                                write_vstring(&mut channel, "zlib")?;
                                verbose.print_verbose("Sent compression list: zlib");

                                let remote_checksum_list = read_vstring(&mut channel)?;
                                verbose.print_verbose(&format!("Received checksum list: {}", remote_checksum_list));

                                let remote_compress_list = read_vstring(&mut channel)?;
                                verbose.print_verbose(&format!("Received compression list: {}", remote_compress_list));
                            } else if negotiated_version >= 30 {
                                verbose.print_verbose("Using default algorithms (no negotiation)");
                            }

                            verbose.print_verbose("Receiving checksum seed...");
                            let mut checksum_seed_bytes = [0u8; 4];
                            channel.read_exact(&mut checksum_seed_bytes)?;
                            let _checksum_seed = i32::from_le_bytes(checksum_seed_bytes);
                            verbose.print_verbose(&format!("Checksum seed: {}", _checksum_seed));

                            let use_multiplex = negotiated_version >= 23;
                            if use_multiplex {
                                verbose.print_verbose("Starting multiplex I/O...");
                                let mut channel = MultiplexIO::new(channel);

                                verbose.print_verbose("Sending filter list...");
                                let exclude_list = ExcludeList::new();
                                exclude_list.send(&mut channel)?;
                                channel.flush()?;
                                verbose.print_verbose("Filter list sent.");

                                Self::handle_multiplexed_protocol(
                                    channel,
                                    is_remote_source,
                                    local_path,
                                    negotiated_version,
                                    &compat_flags,
                                    &self.options,
                                    &verbose,
                                    &mut stats,
                                    start_time
                                )?;

                                return Ok(stats);
                            } else {
                                verbose.print_verbose("Using non-multiplex mode (for debugging)...");

                                verbose.print_verbose("Sending filter list...");
                                let exclude_list = ExcludeList::new();
                                exclude_list.send(&mut channel)?;
                                channel.flush()?;
                                verbose.print_verbose("Filter list sent.");
                            }

                            let local_file_infos = if !is_remote_source {
                                let scanner = Scanner::new()
                                    .recursive(self.options.recursive)
                                    .follow_symlinks(self.options.copy_links);
                                let files = scanner.scan(local_path)?;

                                verbose.print_verbose(&format!("Sending file list ({} files)...", files.len()));
                                send_file_list(&mut channel, &files, local_path, negotiated_version, &compat_flags)?;
                                verbose.print_verbose("File list sent.");

                                files
                            } else {
                                verbose.print_verbose("Skipping file list send (remote source mode)");
                                Vec::new()
                            };

                            verbose.print_verbose("Receiving remote file list...");
                            let remote_file_entries = recv_file_list(&mut channel, negotiated_version, &compat_flags)?;
                            verbose.print_verbose(&format!("Received {} remote files.", remote_file_entries.len()));
                            stats.scanned_files = local_file_infos.len();


                            verbose.print_verbose("Starting file transfer...");

                            if is_remote_source {
                                use crate::protocol::{write_ndx, read_ndx, NdxState, NDX_DONE, write_varint, read_varint};

                                verbose.print_verbose("Acting as generator: requesting files...");
                                let mut ndx_state = NdxState::new();

                                for (idx, remote_entry) in remote_file_entries.iter().enumerate() {
                                    if remote_entry.is_dir {
                                        let dir_path = local_path.join(&remote_entry.path);
                                        if !dir_path.exists() {
                                            verbose.print_verbose(&format!("Creating directory: {}", dir_path.display()));
                                            fs::create_dir_all(&dir_path)?;
                                        }
                                        continue;
                                    }

                                    verbose.print_verbose(&format!("Requesting file {}: {}", idx, remote_entry.path.display()));
                                    write_ndx(&mut channel, idx as i32, &mut ndx_state, negotiated_version)?;

                                    channel.write_i32::<byteorder::LittleEndian>(0)?;
                                    channel.write_i32::<byteorder::LittleEndian>(0)?;
                                    if negotiated_version >= 27 {
                                        channel.write_i32::<byteorder::LittleEndian>(0)?;
                                    }
                                    channel.write_i32::<byteorder::LittleEndian>(0)?;
                                }

                                verbose.print_verbose("Sending NDX_DONE to complete generator phase");
                                write_ndx(&mut channel, NDX_DONE, &mut ndx_state, negotiated_version)?;
                                channel.flush()?;

                                verbose.print_verbose("Acting as receiver: receiving file data...");
                                let mut ndx_state_recv = NdxState::new();

                                loop {
                                    let file_ndx = read_ndx(&mut channel, &mut ndx_state_recv, negotiated_version)?;
                                    if file_ndx == NDX_DONE {
                                        verbose.print_verbose("Received NDX_DONE from sender");
                                        break;
                                    }

                                    if file_ndx < 0 || file_ndx >= remote_file_entries.len() as i32 {
                                        return Err(RsyncError::Other(format!("Invalid file index from sender: {}", file_ndx)));
                                    }

                                    let remote_entry = &remote_file_entries[file_ndx as usize];
                                    verbose.print_basic(&format!("Receiving file {}: {}", file_ndx, remote_entry.path.display()));

                                    let file_path = local_path.join(&remote_entry.path);
                                    if let Some(parent) = file_path.parent() {
                                        if !parent.exists() {
                                            fs::create_dir_all(parent)?;
                                        }
                                    }

                                    let mut file_data = Vec::new();
                                    loop {
                                        let token = read_varint(&mut channel)?;
                                        if token == 0 {
                                            break;
                                        }

                                        if token > 0 {
                                            let len = token as usize;
                                            let mut chunk = vec![0u8; len];
                                            channel.read_exact(&mut chunk)?;
                                            file_data.extend_from_slice(&chunk);
                                        } else {
                                            return Err(RsyncError::Other(format!("Unexpected negative token: {}", token)));
                                        }
                                    }

                                    fs::write(&file_path, &file_data)?;

                                    stats.transferred_files += 1;
                                    stats.transferred_bytes += file_data.len() as u64;

                                    verbose.print_basic(&format!("  Received {} bytes", file_data.len()));
                                }
                            } else {
                                verbose.print_verbose("Sending files to remote...");
                                for local_file in &local_file_infos {
                                    if local_file.is_directory() {
                                        continue;
                                    }

                                    verbose.print_basic(&format!("Sending: {}", local_file.path.display()));

                                    let local_file_path = local_path.join(&local_file.path);
                                    if local_file_path.exists() {
                                        let file_data = fs::read(&local_file_path)?;

                                        use crate::protocol::write_varlong30;
                                        write_varlong30(&mut channel, file_data.len() as i64)?;

                                        channel.write_all(&file_data)?;

                                        stats.transferred_files += 1;
                                        stats.transferred_bytes += file_data.len() as u64;

                                        verbose.print_basic(&format!("  Sent {} bytes", file_data.len()));
                                    }
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
