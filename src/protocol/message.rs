//! rsyncプロトコルで送受信されるメッセージ

/// メッセージタグ（メッセージの種類を識別する）
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageTag {
    /// 通常のデータ
    Data = 0,
    /// エラー
    Error = 1,
    /// 情報メッセージ
    Info = 2,
    /// 警告メッセージ
    Warning = 3,
    /// ファイルリストの開始
    FileListStart = 4,
    /// ファイルリストの終了
    FileListEnd = 5,
    /// プロトコルバージョン
    ProtocolVersion = 7,
    /// ファイルインデックス
    FileIndex = 8,
    /// ファイルチェックサム
    FileChecksum = 9,
    /// ファイルブロック
    FileBlock = 10,
    /// ファイル更新
    FileUpdated = 11,
    /// I/Oタイムアウト
    IoTimeout = 12,
    /// I/Oエラー
    IoError = 13,
    /// その他のメッセージ
    Other = 100,
}

/// rsyncプロトコルメッセージ
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub enum Message {
    /// データブロック
    Data(Vec<u8>),
    /// エラーメッセージ
    Error(String),
    /// 情報メッセージ
    Info(String),
    /// 警告メッセージ
    Warning(String),
    /// ファイルリスト
    FileList(Vec<crate::filesystem::FileInfo>),
    /// 転送終了
    Done,
}

impl Message {
    /// メッセージからタグを取得
    #[allow(dead_code)]
    pub fn tag(&self) -> MessageTag {
        match self {
            Message::Data(_) => MessageTag::Data,
            Message::Error(_) => MessageTag::Error,
            Message::Info(_) => MessageTag::Info,
            Message::Warning(_) => MessageTag::Warning,
            Message::FileList(_) => MessageTag::FileListStart, // 代表として
            Message::Done => MessageTag::FileListEnd, // 代表として
        }
    }
}
