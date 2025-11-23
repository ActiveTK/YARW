#![allow(dead_code)]

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageTag {

    Data = 0,

    Error = 1,

    Info = 2,

    Warning = 3,

    FileListStart = 4,

    FileListEnd = 5,

    ProtocolVersion = 7,

    FileIndex = 8,

    FileChecksum = 9,

    FileBlock = 10,

    FileUpdated = 11,

    IoTimeout = 12,

    IoError = 13,

    Other = 100,
}


#[derive(Debug, Clone)]
pub enum Message {

    Data(Vec<u8>),

    Error(String),

    Info(String),

    Warning(String),

    FileList(Vec<crate::filesystem::FileInfo>),

    Done,
}

impl Message {

    #[allow(dead_code)]
    pub fn tag(&self) -> MessageTag {
        match self {
            Message::Data(_) => MessageTag::Data,
            Message::Error(_) => MessageTag::Error,
            Message::Info(_) => MessageTag::Info,
            Message::Warning(_) => MessageTag::Warning,
            Message::FileList(_) => MessageTag::FileListStart,
            Message::Done => MessageTag::FileListEnd,
        }
    }
}
