mod cli;
mod error;
mod options;
mod filesystem;
mod algorithm;
mod transport;
mod filter;
mod output;
mod protocol;

use clap::Parser;
use cli::Cli;
use error::Result;
use filesystem::path_utils::{is_remote_path, is_daemon_path, parse_remote_path};
use transport::{AuthMethod, DaemonClient, DaemonConfig, RemoteTransport, RsyncDaemon};

#[tokio::main]
async fn main() -> Result<()> {

    env_logger::init();


    let cli = Cli::parse();


    let sources = cli.source.clone();
    let destination = cli.destination.clone();


    let options = cli.into_options()?;

    let verbose = options.verbose_output();

    if let Some(ref log_file_path) = options.log_file {
        match output::init_logger(log_file_path) {
            Ok(_) => {
                verbose.print_basic(&format!("Logging to file: {}", log_file_path.display()));
                output::log_with_timestamp(&format!("YARW (Yet Another Rsync for Windows) v0.1.0 started"));
                output::log(&format!("Command: rsync {} {}", sources.join(" "), destination));
            }
            Err(e) => {
                verbose.print_warning(&format!("Failed to initialize log file: {}", e));
            }
        }
    }


    verbose.print_basic("YARW (Yet Another Rsync for Windows) v0.1.0");
    verbose.print_basic(&format!("Verbose level: {}", options.verbose));


    if options.daemon {
        let config_path = options.config.clone().unwrap_or_else(|| "rsyncd.conf".into());
        let config_str = std::fs::read_to_string(config_path)?;
        let config: DaemonConfig = toml::from_str(&config_str)?;
        let daemon = RsyncDaemon::new(config);
        daemon.start().await?;
        return Ok(());
    }


    let local_transport = transport::LocalTransport::new(options.clone());

    for source_str in &sources {
        let source = std::path::PathBuf::from(source_str);
        let dest = std::path::PathBuf::from(&destination);


        let is_remote_source = is_remote_path(source_str);
        let is_remote_dest = is_remote_path(&destination);
        let is_daemon_source = is_daemon_path(source_str);
        let is_daemon_dest = is_daemon_path(&destination);

        if is_daemon_source || is_daemon_dest {

            if is_daemon_source {

                match DaemonClient::parse_daemon_url(source_str) {
                    Ok((host, port, module, remote_path)) => {
                        verbose.print_basic(&format!("Downloading from rsync daemon: {}:{}/{}", host, port, module));
                        let client = DaemonClient::new(host, port);
                        match client.download(&module, &remote_path, &dest).await {
                            Ok(stats) => {
                                verbose.print_basic(&format!("Download completed: {} files", stats.scanned_files));
                            }
                            Err(e) => {
                                verbose.print_error(&format!("downloading from daemon: {}", e));
                            }
                        }
                    }
                    Err(e) => {
                        verbose.print_error(&format!("parsing daemon URL: {}", e));
                    }
                }
            } else {

                match DaemonClient::parse_daemon_url(&destination) {
                    Ok((host, port, module, remote_path)) => {
                        verbose.print_basic(&format!("Uploading to rsync daemon: {}:{}/{}", host, port, module));
                        let client = DaemonClient::new(host, port);
                        match client.upload(&module, &source, &remote_path).await {
                            Ok(stats) => {
                                verbose.print_basic(&format!("Upload completed: {} files, {} bytes",
                                    stats.transferred_files, stats.transferred_bytes));
                            }
                            Err(e) => {
                                verbose.print_error(&format!("uploading to daemon: {}", e));
                            }
                        }
                    }
                    Err(e) => {
                        verbose.print_error(&format!("parsing daemon URL: {}", e));
                    }
                }
            }
        } else if is_remote_source || is_remote_dest {
            let (user_host, _remote_path) = if is_remote_source {
                parse_remote_path(source_str)
            } else {
                parse_remote_path(&destination)
            };

            if let Some((user, host)) = user_host {
                verbose.print_basic("Remote transfer detected.");
                let username = if user.is_empty() {
                    whoami::username()
                } else {
                    user
                };
                verbose.print_basic(&format!("Connecting to {}@{}...", username, host));


                let _auth_method = AuthMethod::Agent;

                let remote_transport = RemoteTransport::new(options.clone());
                let result = if is_remote_source {
                    remote_transport.sync(source_str, &destination)
                } else {
                    remote_transport.sync(&sources[0], &destination)
                };
                match result {
                    Ok(_) => {
                        verbose.print_basic(&format!("\nRemote sync for {} completed successfully!", source.display()));
                    }
                    Err(e) => {
                        verbose.print_error(&format!("in remote sync for {}: {}", source.display(), e));
                    }
                }
            } else {
                verbose.print_error("Could not parse remote path.");
            }
        } else {
            match local_transport.sync(&source, &dest) {
                Ok(stats) => {
                    if options.stats {
                        stats.display(options.human_readable, &verbose);
                    }
                    verbose.print_basic(&format!("\nSync for {} completed successfully!", source.display()));
                }
                Err(e) => {
                    verbose.print_error(&format!("syncing {}: {}", source.display(), e));
                }
            }
        }
    }

    Ok(())
}
