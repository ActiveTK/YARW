use crate::transport::daemon_config::{DaemonConfig, ModuleConfig};
use crate::protocol::{AsyncProtocolStream, PROTOCOL_VERSION_MAX};
use crate::filesystem::Scanner;
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
        let addr = format!("{}:{}", self.config.address, self.config.port);
        let listener = TcpListener::bind(&addr).await.context(format!("Failed to bind to {}", addr))?;
        println!("Rsync daemon listening on {}", addr);

        loop {
            let (socket, peer_addr) = listener.accept().await?;
            println!("Client connected from: {}", peer_addr);
            let config_clone = self.config.clone();
            tokio::spawn(async move {
                if let Err(e) = Self::handle_client(socket, &config_clone).await {
                    eprintln!("Error handling client {}: {}", peer_addr, e);
                }
            });
        }
    }

    async fn handle_client(socket: TcpStream, config: &DaemonConfig) -> Result<()> {
        let mut stream = AsyncProtocolStream::new(socket, PROTOCOL_VERSION_MAX);

        // 1. プロトコルバージョン交渉
        println!("Negotiating protocol version...");
        let client_version = stream.read_i32().await?;
        println!("Client version: {}", client_version);

        // サーバーバージョンを送信
        stream.write_i32(PROTOCOL_VERSION_MAX).await?;
        stream.flush().await?;

        // 2回目のバージョン交換（rsyncプロトコル）
        stream.write_i32(PROTOCOL_VERSION_MAX).await?;
        stream.flush().await?;
        let _client_version_ack = stream.read_i32().await?;

        // 2. モジュール名を受信
        println!("Waiting for module name...");
        let module_name = stream.read_string(256).await?;
        println!("Client requested module: {}", module_name);

        // 3. モジュール設定を取得
        let module_config = config.modules.get(&module_name)
            .ok_or_else(|| anyhow::anyhow!("Module '{}' not found", module_name))?;

        // 4. 認証（必要な場合）
        if let Some(ref auth_users) = module_config.auth_users {
            println!("Authentication required for module '{}'", module_name);
            if !Self::authenticate(&mut stream, auth_users, &module_config).await? {
                bail!("Authentication failed");
            }
            println!("Authentication successful");
        }

        // 5. ファイル転送処理
        Self::handle_file_transfer(&mut stream, module_config).await?;

        println!("Client session completed successfully");
        Ok(())
    }

    async fn authenticate(
        stream: &mut AsyncProtocolStream<TcpStream>,
        _auth_users: &[String],
        module_config: &ModuleConfig,
    ) -> Result<bool> {
        // 認証チャレンジを送信
        stream.write_string("@RSYNCD: AUTHREQD").await?;
        stream.flush().await?;

        // ユーザー名を受信
        let username = stream.read_string(256).await?;
        println!("Authentication attempt for user: {}", username);

        // パスワード（ハッシュ）を受信
        let password_hash = stream.read_string(512).await?;

        // パスワードファイルから検証
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
                            // 簡易的なパスワード検証（本来はMD4ハッシュ比較が必要）
                            if password_hash == file_pass {
                                return Ok(true);
                            }
                        }
                    }
                }
            }
        }

        // 認証失敗
        stream.write_string("@RSYNCD: AUTH FAILED").await?;
        stream.flush().await?;
        Ok(false)
    }

    async fn handle_file_transfer(
        stream: &mut AsyncProtocolStream<TcpStream>,
        module_config: &ModuleConfig,
    ) -> Result<()> {
        println!("Starting file transfer for path: {:?}", module_config.path);

        // ファイルリストをスキャン
        let scanner = Scanner::new().recursive(true);
        let files = scanner.scan(&module_config.path)?;
        println!("Scanned {} files", files.len());

        // ファイル数を送信
        stream.write_varint(files.len() as i64).await?;

        // 各ファイルの情報を送信
        for file in &files {
            // ファイル名
            let relative_path = file.path.strip_prefix(&module_config.path)
                .unwrap_or(&file.path);
            stream.write_string(&relative_path.to_string_lossy()).await?;

            // ファイルサイズ
            stream.write_varint(file.size as i64).await?;

            // 修正時刻（UNIX時間）
            let mtime_secs = file.mtime.duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            stream.write_varint(mtime_secs as i64).await?;

            // ファイルタイプ
            let file_type_code = if file.is_directory() { 1i8 } else { 0i8 };
            stream.write_i8(file_type_code).await?;
        }

        stream.flush().await?;
        println!("File list sent");

        // read_onlyでない場合、クライアントからのファイルを受信
        if !module_config.read_only {
            println!("Receiving files from client...");
            // クライアントからのファイルデータを受信
            let num_files = stream.read_varint().await? as usize;
            println!("Client sending {} files", num_files);

            for i in 0..num_files {
                let file_path = stream.read_string(4096).await?;
                let file_size = stream.read_varint().await? as usize;

                println!("Receiving file {}: {} ({} bytes)", i + 1, file_path, file_size);

                let dest_path = module_config.path.join(&file_path);

                // ディレクトリを作成
                if let Some(parent) = dest_path.parent() {
                    fs::create_dir_all(parent)?;
                }

                // ファイルデータを受信
                let mut file_data = vec![0u8; file_size];
                stream.read_all(&mut file_data).await?;
                fs::write(&dest_path, &file_data)?;

                println!("Saved file: {:?}", dest_path);
            }
        }

        println!("File transfer completed");
        Ok(())
    }
}
