//! rsyncプロトコル層
//!
//! リモートホストとの通信プロトコルを扱う。
//! - バージョン交渉
//! - バイトストリームのエンコード/デコード
//! - ファイルリストの送受信

pub mod version;
pub mod stream;
pub mod async_stream;
pub mod message;
pub mod file_list;

pub use version::PROTOCOL_VERSION_MAX;
pub use stream::ProtocolStream;
pub use async_stream::AsyncProtocolStream;
pub use file_list::FileList;

// 将来使用する予定のエクスポート
#[allow(unused_imports)]
pub use version::{ProtocolVersion, PROTOCOL_VERSION_MIN};
#[allow(unused_imports)]
pub use message::Message;
