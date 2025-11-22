use crate::options::Options;
use crate::error::{Result, RsyncError};
use super::{SshTransport, AuthMethod, SyncStats};
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
            // -e オプションからSSHパラメータを抽出
            let (port, auth_method) = if let Some(ref rsh_command) = self.options.rsh {
                let params = parse_ssh_command(rsh_command);
                let port = params.port.unwrap_or(22);
                let auth_method = if let Some(identity_file) = params.identity_file {
                    AuthMethod::PublicKey(identity_file)
                } else {
                    AuthMethod::Agent
                };
                (port, auth_method)
            } else {
                (22, AuthMethod::Agent)
            };

            println!("Connecting to {}@{}:{} ...", username, host, port);

            match SshTransport::connect(&host, port, &username, auth_method) {
                Ok(mut transport) => {
                    println!("SSH connection successful.");

                    // リモートのrsyncパスをUnix形式に変換
                    let remote_unix_path = to_unix_separators(&remote_raw_path);
                    
                    // rsync --server コマンドを構築
                    let mut rsync_args = vec![
                        "--server",
                        "--sender", // 今回はローカルがsenderとして振る舞う
                    ];
                    // その他のrsyncオプションをここに含める
                    // 例: -v, -r, --delete など
                    if self.options.recursive { rsync_args.push("-r"); }
                    if self.options.verbose > 0 { rsync_args.push("-v"); }
                    if self.options.delete { rsync_args.push("--delete"); }
                    
                    rsync_args.push("."); // ソースディレクトリ
                    rsync_args.push(&remote_unix_path); // デスティネーションディレクトリ

                    let rsync_command_str = format!("rsync {}", rsync_args.join(" "));
                    println!("Executing remote command: {}", rsync_command_str);

                    match transport.execute(&rsync_command_str) {
                        Ok(mut channel) => {
                            // SSHチャネルをProtocolStreamでラップ
                            let mut stream = ProtocolStream::new(&mut channel, PROTOCOL_VERSION_MAX);

                            // 1. プロトコルバージョン交渉 (rsyncの規約に従う)
                            // クライアント側でローカルのrsyncバージョンを送信し、リモートからの応答を待つ
                            // 現状はダミーのバージョンを送信し、リモートも同じバージョンと仮定
                            println!("Negotiating protocol version...");
                            stream.write_i32(PROTOCOL_VERSION_MAX)?; // クライアントが送信
                            stream.flush()?;
                            let remote_version = stream.read_i32()?; // リモートが受信して応答

                            // 本家rsyncはバージョン番号を2回送るため、ここでも合わせる
                            stream.write_i32(PROTOCOL_VERSION_MAX)?;
                            stream.flush()?;
                            let _remote_version_ack = stream.read_i32()?;
                            
                            println!("Negotiated protocol version: {}", remote_version);

                            // 2. ローカルファイルをスキャン
                            let scanner = Scanner::new()
                                .recursive(self.options.recursive)
                                .follow_symlinks(self.options.copy_links);
                            let local_file_infos = scanner.scan(local_path)?;
                            
                            // 3. ファイルリストの送信
                            println!("Sending file list...");
                            FileList::encode(&mut stream, &local_file_infos)?;
                            println!("File list sent.");

                            // 4. リモートからのファイルリスト受信
                            println!("Receiving remote file list...");
                            let remote_file_infos = FileList::decode(&mut stream)?;
                            println!("Received {} remote files.", remote_file_infos.len());
                            stats.scanned_files = local_file_infos.len();

                            // 5. ファイル転送処理
                            println!("Starting file transfer...");

                            // 各ローカルファイルについて処理
                            for local_file in &local_file_infos {
                                if local_file.is_directory() {
                                    // ディレクトリはスキップ（リモートで作成される）
                                    continue;
                                }

                                // リモートファイルの存在確認
                                let remote_file = remote_file_infos.iter()
                                    .find(|f| f.path == local_file.path);

                                if self.options.verbose > 0 {
                                    println!("Processing: {}", local_file.path.display());
                                }

                                // 簡易実装: 常に全ファイル転送を使用
                                // (完全なrsync差分転送アルゴリズムは将来の拡張として残す)
                                if self.options.verbose > 1 {
                                    if remote_file.is_some() {
                                        println!("  Updating existing file (whole-file transfer)");
                                    } else {
                                        println!("  New file");
                                    }
                                }

                                // ファイルデータの送信（簡易実装）
                                let local_file_path = local_path.join(&local_file.path);
                                if local_file_path.exists() {
                                    let file_data = fs::read(&local_file_path)?;

                                    // データサイズを送信
                                    stream.write_varint(file_data.len() as i64)?;

                                    // データを送信
                                    stream.write_all(&file_data)?;
                                    stream.flush()?;

                                    stats.transferred_files += 1;
                                    stats.transferred_bytes += file_data.len() as u64;

                                    if self.options.verbose > 0 {
                                        println!("  Transferred {} bytes", file_data.len());
                                    }
                                }
                            }

                            // 6. 統計情報を計算
                            stats.execution_time_secs = start_time.elapsed().as_secs_f64();

                            println!("Transfer complete!");
                            if self.options.stats {
                                stats.display(self.options.human_readable);
                            }

                            // エラー出力を読み取る
                            let mut stderr_bytes = Vec::new();
                            match channel.stderr().read_to_end(&mut stderr_bytes) {
                                Ok(_) => {
                                    if !stderr_bytes.is_empty() {
                                        eprintln!("Remote stderr: {}", String::from_utf8_lossy(&stderr_bytes));
                                    }
                                },
                                Err(e) => eprintln!("Failed to read remote stderr: {}", e),
                            }

                            // チャネルを閉じる
                            channel.close()?;
                            channel.wait_close()?;

                        }
                        Err(e) => return Err(RsyncError::RemoteExec(format!("Failed to execute remote command: {}", e))),
                    }
                }
                Err(e) => return Err(RsyncError::Network(format!("SSH connection failed: {}", e))),
            }
        } else {
            return Err(RsyncError::InvalidPath(PathBuf::from(source))); // リモートパスとして認識できない
        }

        Ok(stats)
    }
}