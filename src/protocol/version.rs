use crate::error::{Result, RsyncError};


pub const PROTOCOL_VERSION_MIN: i32 = 27;
pub const PROTOCOL_VERSION_MAX: i32 = 31;


#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProtocolVersion {
    pub version: i32,
}

impl ProtocolVersion {

    #[allow(dead_code)]
    pub fn new(version: i32) -> Self {
        Self { version }
    }










    #[allow(dead_code)]
    pub fn negotiate(local_version: i32, remote_version: i32) -> Result<i32> {

        let version = local_version.min(remote_version);

        if version >= PROTOCOL_VERSION_MIN {
            Ok(version)
        } else {
            Err(RsyncError::IncompatibleProtocol {
                local: local_version,
                remote: remote_version,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_negotiate_success() {

        assert_eq!(ProtocolVersion::negotiate(31, 27).unwrap(), 27);


        assert_eq!(ProtocolVersion::negotiate(28, 30).unwrap(), 28);


        assert_eq!(ProtocolVersion::negotiate(29, 29).unwrap(), 29);


        assert_eq!(ProtocolVersion::negotiate(PROTOCOL_VERSION_MAX, PROTOCOL_VERSION_MAX).unwrap(), PROTOCOL_VERSION_MAX);
    }

    #[test]
    fn test_negotiate_failure_remote_too_old() {
        let result = ProtocolVersion::negotiate(31, 26);
        assert!(result.is_err());
        match result.unwrap_err() {
            RsyncError::IncompatibleProtocol { local, remote } => {
                assert_eq!(local, 31);
                assert_eq!(remote, 26);
            }
            _ => panic!("Expected IncompatibleProtocol error"),
        }
    }

    #[test]
    fn test_negotiate_failure_local_too_old() {

        let result = ProtocolVersion::negotiate(25, 28);
        assert!(result.is_err());
    }
}
