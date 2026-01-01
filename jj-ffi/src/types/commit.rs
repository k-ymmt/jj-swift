//! Commit type for FFI

use jj_lib::commit::Commit;

use super::ids::{FfiChangeId, FfiCommitId};
use super::signature::{FfiSignature, FfiTimestamp};

/// A commit exposed via FFI
#[derive(Debug, Clone, uniffi::Record)]
pub struct FfiCommit {
    /// The commit ID (content-based hash)
    pub id: FfiCommitId,
    /// The change ID (stable identifier across rewrites)
    pub change_id: FfiChangeId,
    /// Commit description/message
    pub description: String,
    /// Author signature
    pub author: FfiSignature,
    /// Committer signature
    pub committer: FfiSignature,
    /// Parent commit IDs
    pub parent_ids: Vec<FfiCommitId>,
    /// Whether this commit is signed
    pub is_signed: bool,
}

impl From<&Commit> for FfiCommit {
    fn from(commit: &Commit) -> Self {
        Self {
            id: FfiCommitId::from(commit.id()),
            change_id: FfiChangeId::from(commit.change_id()),
            description: commit.description().to_string(),
            author: FfiSignature::from(commit.author()),
            committer: FfiSignature::from(commit.committer()),
            parent_ids: commit.parent_ids().iter().map(FfiCommitId::from).collect(),
            is_signed: commit.is_signed(),
        }
    }
}

impl From<Commit> for FfiCommit {
    fn from(commit: Commit) -> Self {
        Self::from(&commit)
    }
}

/// Input data for creating a new commit via FFI
#[derive(Debug, Clone, uniffi::Record)]
pub struct FfiNewCommit {
    /// Parent commit IDs
    pub parent_ids: Vec<FfiCommitId>,
    /// Commit description/message
    pub description: String,
    /// Author name (optional, uses settings default if not provided)
    pub author_name: Option<String>,
    /// Author email (optional, uses settings default if not provided)
    pub author_email: Option<String>,
    /// Author timestamp (optional, uses current time if not provided)
    pub author_timestamp: Option<FfiTimestamp>,
}

impl FfiNewCommit {
    /// Create a new commit builder with minimal required fields
    pub fn new(parent_ids: Vec<FfiCommitId>, description: String) -> Self {
        Self {
            parent_ids,
            description,
            author_name: None,
            author_email: None,
            author_timestamp: None,
        }
    }
}

/// Input data for rewriting an existing commit via FFI
#[derive(Debug, Clone, uniffi::Record)]
pub struct FfiRewriteCommit {
    /// The commit ID to rewrite
    pub commit_id: FfiCommitId,
    /// New description (optional, keeps original if not provided)
    pub new_description: Option<String>,
    /// New parent IDs (optional, keeps original if not provided)
    pub new_parent_ids: Option<Vec<FfiCommitId>>,
}
