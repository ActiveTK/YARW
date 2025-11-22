use crate::protocol::{AsyncProtocolStream, PROTOCOL_VERSION_MAX};
use crate::filesystem::{Scanner, FileInfo, FileType};
use crate::transport::SyncStats;
use tokio::net::TcpStream;
use anyhow::{Result, Context, bail};
use std::path::{Path, PathBuf};
use std::time::Instant;
use std::fs;

/// rsyncデーモンクライアント
pub struct DaemonClient {
    host: String,
    port: u16,
}

impl DaemonClient {
    pub fn new(host: String, port: u16) -> Self {
        Self { host, port }
    }

    /// rsync://host:port/module/path 形式のURLをパース
    pub fn parse_daemon_url(url: &str) -> Result<(String, u16, String, String)> {
        // rsync://host:port/module/path
        if !url.starts_with("rsync://") {
            bail!("Invalid daemon URL: must start with rsync://");
        }

        let without_protocol = &url[8..]; // "rsync://" を削除
        let parts: Vec<&str> = without_protocol.splitn(2, '/').collect();

        if parts.len() < 2 {
            bail!("Invalid daemon URL: missing module");
        }

        let host_port = parts[0];
        let module_and_path = parts[1];

        // ホストとポートを分離
        let (host, port) = if host_port.contains(':') {
            let hp: Vec<&str> = host_port.splitn(2, ':').collect();
            (hp[0].to_string(), hp[1].parse::<u16>()?)
        } else {
            (host_port.to_string(), 873) // デフォルトポート
        };

        // モジュールとパスを分離
        let mp_parts: Vec<&str> = module_and_path.splitn(2, '/').collect();
        let module = mp_parts[0].to_string();
        let path = if mp_parts.len() > 1 {
            mp_parts[1].to_string()
        } else {
            String::new()
        };

        Ok((host, port, module, path))
    }

    /// デーモンに接続してファイルを取得（ダウンロード）
    pub async fn download(
        &self,
        module: &str,
        _remote_path: &str,
        _local_path: &Path,
    ) -> Result<SyncStats> {
        let start_time = Instant::now();
        let mut stats = SyncStats::default();

        // サーバーに接続
        let addr = format!("{}:{}", self.host, self.port);
        let socket = TcpStream::connect(&addr).await
            .context(format!("Failed to connect to {}", addr))?;
        println!("Connected to rsync daemon at {}", addr);

        let mut stream = AsyncProtocolStream::new(socket, PROTOCOL_VERSION_MAX);

        // 1. プロトコルバージョン交渉
        stream.write_i32(PROTOCOL_VERSION_MAX).await?;
        stream.flush().await?;

        let server_version = stream.read_i32().await?;
        println!("Server version: {}", server_version);

        // 2回目のバージョン交換
        let _server_version_ack = stream.read_i32().await?;
        stream.write_i32(PROTOCOL_VERSION_MAX).await?;
        stream.flush().await?;

        // 2. モジュール名を送信
        stream.write_string(module).await?;
        stream.flush().await?;
        println!("Requested module: {}", module);

        // 3. 認証（必要な場合）
        // サーバーから認証要求が来る可能性がある
        // 簡略化のため、今回は認証なしを想定

        // 4. ファイルリストを受信
        let num_files = stream.read_varint().await? as usize;
        println!("Receiving {} files from server", num_files);

        let mut files = Vec::with_capacity(num_files);
        for _ in 0..num_files {
            let file_path = stream.read_string(4096).await?;
            let file_size = stream.read_varint().await? as u64;
            let mtime_secs = stream.read_varint().await? as u64;
            let file_type_code = stream.read_i8().await?;

            let file_type = match file_type_code {
                0 => FileType::File,
                1 => FileType::Directory,
                _ => FileType::File,
            };

            let mtime = std::time::UNIX_EPOCH + std::time::Duration::from_secs(mtime_secs);

            let file_info = FileInfo {
                path: PathBuf::from(&file_path),
                size: file_size,
                mtime,
                file_type,
                is_symlink: false,
                symlink_target: None,
            };

            files.push(file_info);
        }

        println!("Received file list with {} entries", files.len());
        stats.scanned_files = files.len();

        // 5. ファイルをローカルに保存
        // （簡易実装：ファイルリストのみ受信、実際のデータ転送は省略）
        // 実際のrsyncでは、ここでファイルデータを受信する

        stats.execution_time_secs = start_time.elapsed().as_secs_f64();
        println!("Download completed in {:.2}s", stats.execution_time_secs);

        Ok(stats)
    }

    /// デーモンにファイルをアップロード
    pub async fn upload(
        &self,
        module: &str,
        local_path: &Path,
        _remote_path: &str,
    ) -> Result<SyncStats> {
        let start_time = Instant::now();
        let mut stats = SyncStats::default();

        // サーバーに接続
        let addr = format!("{}:{}", self.host, self.port);
        let socket = TcpStream::connect(&addr).await
            .context(format!("Failed to connect to {}", addr))?;
        println!("Connected to rsync daemon at {}", addr);

        let mut stream = AsyncProtocolStream::new(socket, PROTOCOL_VERSION_MAX);

        // 1. プロトコルバージョン交渉
        stream.write_i32(PROTOCOL_VERSION_MAX).await?;
        stream.flush().await?;

        let server_version = stream.read_i32().await?;
        println!("Server version: {}", server_version);

        // 2回目のバージョン交換
        let _server_version_ack = stream.read_i32().await?;
        stream.write_i32(PROTOCOL_VERSION_MAX).await?;
        stream.flush().await?;

        // 2. モジュール名を送信
        stream.write_string(module).await?;
        stream.flush().await?;

        // 3. サーバーからファイルリストを受信（スキップ）
        let num_server_files = stream.read_varint().await? as usize;
        println!("Server has {} files", num_server_files);

        // サーバーファイルリストをスキップ
        for _ in 0..num_server_files {
            let _file_path = stream.read_string(4096).await?;
            let _file_size = stream.read_varint().await?;
            let _mtime = stream.read_varint().await?;
            let _file_type = stream.read_i8().await?;
        }

        // 4. ローカルファイルをスキャン
        let scanner = Scanner::new().recursive(true);
        let local_files = scanner.scan(local_path)?;
        println!("Uploading {} files to server", local_files.len());

        // 5. ファイル数を送信
        stream.write_varint(local_files.len() as i64).await?;

        // 6. 各ファイルを送信
        for file in &local_files {
            if file.is_directory() {
                continue; // ディレクトリはスキップ
            }

            let relative_path = file.path.strip_prefix(local_path)
                .unwrap_or(&file.path);

            // ファイル名
            stream.write_string(&relative_path.to_string_lossy()).await?;

            // ファイルデータを読み込み
            let file_path = local_path.join(&file.path);
            let file_data = fs::read(&file_path)?;

            // ファイルサイズ
            stream.write_varint(file_data.len() as i64).await?;

            // ファイルデータ
            stream.write_all(&file_data).await?;

            stats.transferred_files += 1;
            stats.transferred_bytes += file_data.len() as u64;

            println!("Uploaded: {} ({} bytes)", relative_path.display(), file_data.len());
        }

        stream.flush().await?;

        stats.scanned_files = local_files.len();
        stats.execution_time_secs = start_time.elapsed().as_secs_f64();

        println!("Upload completed in {:.2}s", stats.execution_time_secs);
        println!("Transferred {} files, {} bytes", stats.transferred_files, stats.transferred_bytes);

        Ok(stats)
    }
}
