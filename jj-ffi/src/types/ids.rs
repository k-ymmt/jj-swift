//! Commit and Change ID types for FFI

use jj_lib::backend::{ChangeId, CommitId};
use jj_lib::object_id::ObjectId;

/// A commit ID represented as a hex string for FFI
#[derive(Debug, Clone, PartialEq, Eq, Hash, uniffi::Record)]
pub struct FfiCommitId {
    /// Hex-encoded commit ID
    pub hex: String,
}

impl FfiCommitId {
    pub fn new(hex: String) -> Self {
        Self { hex }
    }
}

impl From<&CommitId> for FfiCommitId {
    fn from(id: &CommitId) -> Self {
        Self { hex: id.hex() }
    }
}

impl From<CommitId> for FfiCommitId {
    fn from(id: CommitId) -> Self {
        Self::from(&id)
    }
}

impl TryFrom<&FfiCommitId> for CommitId {
    type Error = hex::FromHexError;

    fn try_from(ffi_id: &FfiCommitId) -> Result<Self, Self::Error> {
        let bytes = hex::decode(&ffi_id.hex)?;
        Ok(CommitId::new(bytes))
    }
}

/// A change ID represented as a reverse-hex string for FFI
#[derive(Debug, Clone, PartialEq, Eq, Hash, uniffi::Record)]
pub struct FfiChangeId {
    /// Reverse-hex encoded change ID (z-k digits)
    pub hex: String,
}

impl FfiChangeId {
    pub fn new(hex: String) -> Self {
        Self { hex }
    }
}

impl From<&ChangeId> for FfiChangeId {
    fn from(id: &ChangeId) -> Self {
        Self {
            hex: id.reverse_hex(),
        }
    }
}

impl From<ChangeId> for FfiChangeId {
    fn from(id: ChangeId) -> Self {
        Self::from(&id)
    }
}

impl TryFrom<&FfiChangeId> for ChangeId {
    type Error = crate::error::JjError;

    fn try_from(ffi_id: &FfiChangeId) -> Result<Self, Self::Error> {
        ChangeId::try_from_reverse_hex(&ffi_id.hex).ok_or_else(|| crate::error::JjError::InvalidArgument {
            message: format!("Invalid change ID: {}", ffi_id.hex),
        })
    }
}
