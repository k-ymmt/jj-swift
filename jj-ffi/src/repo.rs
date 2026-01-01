//! Repository operations for FFI

use std::sync::Arc;

use jj_lib::backend::CommitId;
#[cfg(feature = "git")]
use jj_lib::git::GitSettings;
use jj_lib::object_id::{HexPrefix, PrefixResolution};
use jj_lib::repo::{ReadonlyRepo, Repo};

use crate::error::{JjError, Result};
#[cfg(feature = "git")]
use crate::git::FfiGitTransaction;
use crate::transaction::FfiTransaction;
use crate::types::{FfiChangeId, FfiCommit, FfiCommitId};

/// A readonly repository exposed via FFI
#[derive(uniffi::Object)]
pub struct FfiReadonlyRepo {
    inner: Arc<ReadonlyRepo>,
}

impl FfiReadonlyRepo {
    pub fn new(repo: Arc<ReadonlyRepo>) -> Self {
        Self { inner: repo }
    }

    pub fn inner(&self) -> &Arc<ReadonlyRepo> {
        &self.inner
    }
}

#[uniffi::export]
impl FfiReadonlyRepo {
    /// Get a commit by its commit ID (hex string)
    pub fn get_commit(&self, commit_id: &FfiCommitId) -> Result<FfiCommit> {
        let id = CommitId::try_from(commit_id).map_err(|e| JjError::InvalidArgument {
            message: format!("Invalid commit ID: {}", e),
        })?;

        let commit = self.inner.store().get_commit(&id)?;
        Ok(FfiCommit::from(&commit))
    }

    /// Get the root commit of the repository
    pub fn root_commit(&self) -> FfiCommit {
        let commit = self.inner.store().root_commit();
        FfiCommit::from(&commit)
    }

    /// Get the root commit ID
    pub fn root_commit_id(&self) -> FfiCommitId {
        FfiCommitId::from(self.inner.store().root_commit_id())
    }

    /// Get the root change ID
    pub fn root_change_id(&self) -> FfiChangeId {
        FfiChangeId::from(self.inner.store().root_change_id())
    }

    /// Resolve a change ID to commit IDs
    pub fn resolve_change_id(&self, change_id: &FfiChangeId) -> Result<Vec<FfiCommitId>> {
        let id = jj_lib::backend::ChangeId::try_from(change_id)?;
        let maybe_commit_ids = self.inner.resolve_change_id(&id).map_err(|e| {
            JjError::Internal {
                message: format!("Index error: {}", e),
            }
        })?;

        match maybe_commit_ids {
            Some(commit_ids) => Ok(commit_ids.into_iter().map(FfiCommitId::from).collect()),
            None => Err(JjError::CommitNotFound {
                id: change_id.hex.clone(),
            }),
        }
    }

    /// Resolve a commit ID prefix (returns all matching commits)
    pub fn resolve_commit_prefix(&self, prefix: &str) -> Result<Vec<FfiCommitId>> {
        let hex_prefix = HexPrefix::try_from_hex(prefix).ok_or_else(|| JjError::InvalidArgument {
            message: format!("Invalid hex prefix: {}", prefix),
        })?;

        let resolution = self
            .inner
            .index()
            .resolve_commit_id_prefix(&hex_prefix)
            .map_err(|e| JjError::Internal {
                message: format!("Index error: {}", e),
            })?;

        match resolution {
            PrefixResolution::NoMatch => Err(JjError::CommitNotFound {
                id: prefix.to_string(),
            }),
            PrefixResolution::SingleMatch(id) => Ok(vec![FfiCommitId::from(&id)]),
            PrefixResolution::AmbiguousMatch => Err(JjError::InvalidArgument {
                message: format!("Ambiguous commit prefix: {}", prefix),
            }),
        }
    }

    /// Evaluate a revset expression and return matching commit IDs
    pub fn evaluate_revset(&self, revset_str: String, user_email: String) -> Result<Vec<FfiCommitId>> {
        crate::revset::evaluate_revset(&self.inner, &revset_str, &user_email)
    }

    /// Evaluate a revset expression and return matching commits
    pub fn evaluate_revset_to_commits(&self, revset_str: String, user_email: String) -> Result<Vec<FfiCommit>> {
        crate::revset::evaluate_revset_to_commits(&self.inner, &revset_str, &user_email)
    }

    /// Count commits matching a revset expression
    pub fn count_revset(&self, revset_str: String, user_email: String) -> Result<u64> {
        crate::revset::count_revset(&self.inner, &revset_str, &user_email)
    }

    /// Start a new transaction for making changes to the repository
    pub fn start_transaction(&self) -> Arc<FfiTransaction> {
        let tx = self.inner.start_transaction();
        Arc::new(FfiTransaction::new(tx))
    }

    /// Start a new Git transaction for performing Git operations
    #[cfg(feature = "git")]
    pub fn start_git_transaction(&self) -> Result<Arc<FfiGitTransaction>> {
        let settings = self.inner.settings();
        let git_settings = GitSettings::from_settings(settings).map_err(|e| JjError::Git {
            message: format!("Failed to load Git settings: {}", e),
        })?;
        let tx = self.inner.start_transaction();
        Ok(Arc::new(FfiGitTransaction::new(tx, git_settings)))
    }
}
