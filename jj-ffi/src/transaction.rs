//! Transaction operations for FFI

use std::sync::{Arc, Mutex};

use jj_lib::backend::{CommitId, Signature, Timestamp};
use jj_lib::repo::Repo;
use jj_lib::transaction::Transaction;

use crate::error::{JjError, Result};
use crate::repo::FfiReadonlyRepo;
use crate::types::{FfiCommit, FfiCommitId, FfiNewCommit, FfiRewriteCommit};

/// A transaction for making changes to a repository
///
/// # Safety
/// The Transaction type from jj-lib is not Send + Sync, but we wrap it in a Mutex
/// to ensure synchronized access. All FFI operations acquire the lock before accessing
/// the transaction.
#[derive(uniffi::Object)]
pub struct FfiTransaction {
    inner: Mutex<Option<Transaction>>,
}

// SAFETY: FfiTransaction is protected by a Mutex, ensuring synchronized access
// across threads. The internal Transaction is only accessed through the Mutex.
unsafe impl Send for FfiTransaction {}
unsafe impl Sync for FfiTransaction {}

impl FfiTransaction {
    pub(crate) fn new(transaction: Transaction) -> Self {
        Self {
            inner: Mutex::new(Some(transaction)),
        }
    }

    fn with_transaction<T, F>(&self, f: F) -> Result<T>
    where
        F: FnOnce(&Transaction) -> Result<T>,
    {
        let guard = self.inner.lock().map_err(|_| JjError::Internal {
            message: "Failed to acquire transaction lock".to_string(),
        })?;
        let tx = guard.as_ref().ok_or_else(|| JjError::Internal {
            message: "Transaction has already been committed or discarded".to_string(),
        })?;
        f(tx)
    }

    fn with_transaction_mut<T, F>(&self, f: F) -> Result<T>
    where
        F: FnOnce(&mut Transaction) -> Result<T>,
    {
        let mut guard = self.inner.lock().map_err(|_| JjError::Internal {
            message: "Failed to acquire transaction lock".to_string(),
        })?;
        let tx = guard.as_mut().ok_or_else(|| JjError::Internal {
            message: "Transaction has already been committed or discarded".to_string(),
        })?;
        f(tx)
    }

    fn take_transaction(&self) -> Result<Transaction> {
        let mut guard = self.inner.lock().map_err(|_| JjError::Internal {
            message: "Failed to acquire transaction lock".to_string(),
        })?;
        guard.take().ok_or_else(|| JjError::Internal {
            message: "Transaction has already been committed or discarded".to_string(),
        })
    }
}

#[uniffi::export]
impl FfiTransaction {
    /// Create a new commit with an empty tree (for creating empty commits)
    pub fn create_empty_commit(&self, new_commit: FfiNewCommit) -> Result<FfiCommit> {
        self.with_transaction_mut(|tx| {
            let store = tx.repo().store();

            // Convert parent IDs
            let parent_ids: Vec<CommitId> = new_commit
                .parent_ids
                .iter()
                .map(|id| CommitId::try_from(id))
                .collect::<std::result::Result<Vec<_>, _>>()
                .map_err(|e| JjError::InvalidArgument {
                    message: format!("Invalid parent commit ID: {}", e),
                })?;

            if parent_ids.is_empty() {
                return Err(JjError::InvalidArgument {
                    message: "At least one parent commit ID is required".to_string(),
                });
            }

            // Get empty tree
            let tree = store.empty_merged_tree();

            // Create the commit
            let mut builder = tx.repo_mut().new_commit(parent_ids, tree);
            builder = builder.set_description(&new_commit.description);

            // Set author if provided
            if let (Some(name), Some(email)) = (&new_commit.author_name, &new_commit.author_email) {
                let timestamp = new_commit
                    .author_timestamp
                    .map(Timestamp::from)
                    .unwrap_or_else(Timestamp::now);
                let author = Signature {
                    name: name.clone(),
                    email: email.clone(),
                    timestamp,
                };
                builder = builder.set_author(author);
            }

            let commit = builder.write().map_err(|e| JjError::Backend {
                message: e.to_string(),
            })?;

            Ok(FfiCommit::from(&commit))
        })
    }

    /// Create a new commit with the same tree as a parent commit
    pub fn create_commit_from_parent(
        &self,
        parent_id: &FfiCommitId,
        description: String,
    ) -> Result<FfiCommit> {
        self.with_transaction_mut(|tx| {
            // Convert parent ID
            let parent_commit_id =
                CommitId::try_from(parent_id).map_err(|e| JjError::InvalidArgument {
                    message: format!("Invalid parent commit ID: {}", e),
                })?;

            // Get the parent commit
            let parent_commit = tx
                .repo()
                .store()
                .get_commit(&parent_commit_id)
                .map_err(|e| JjError::Backend {
                    message: e.to_string(),
                })?;

            // Use the parent's tree
            let tree = parent_commit.tree();

            // Create the commit
            let builder = tx
                .repo_mut()
                .new_commit(vec![parent_commit_id], tree)
                .set_description(&description);

            let commit = builder.write().map_err(|e| JjError::Backend {
                message: e.to_string(),
            })?;

            Ok(FfiCommit::from(&commit))
        })
    }

    /// Rewrite an existing commit with new properties
    pub fn rewrite_commit(&self, rewrite: FfiRewriteCommit) -> Result<FfiCommit> {
        self.with_transaction_mut(|tx| {
            // Convert commit ID
            let commit_id =
                CommitId::try_from(&rewrite.commit_id).map_err(|e| JjError::InvalidArgument {
                    message: format!("Invalid commit ID: {}", e),
                })?;

            // Get the commit to rewrite
            let commit = tx
                .repo()
                .store()
                .get_commit(&commit_id)
                .map_err(|e| JjError::Backend {
                    message: e.to_string(),
                })?;

            // Create rewrite builder
            let mut builder = tx.repo_mut().rewrite_commit(&commit);

            // Apply changes
            if let Some(desc) = &rewrite.new_description {
                builder = builder.set_description(desc);
            }

            if let Some(new_parent_ids) = &rewrite.new_parent_ids {
                let parent_ids: Vec<CommitId> = new_parent_ids
                    .iter()
                    .map(|id| CommitId::try_from(id))
                    .collect::<std::result::Result<Vec<_>, _>>()
                    .map_err(|e| JjError::InvalidArgument {
                        message: format!("Invalid parent commit ID: {}", e),
                    })?;

                if parent_ids.is_empty() {
                    return Err(JjError::InvalidArgument {
                        message: "At least one parent commit ID is required".to_string(),
                    });
                }

                builder = builder.set_parents(parent_ids);
            }

            let new_commit = builder.write().map_err(|e| JjError::Backend {
                message: e.to_string(),
            })?;

            Ok(FfiCommit::from(&new_commit))
        })
    }

    /// Update the description of an existing commit
    pub fn update_description(
        &self,
        commit_id: &FfiCommitId,
        new_description: String,
    ) -> Result<FfiCommit> {
        let rewrite = FfiRewriteCommit {
            commit_id: commit_id.clone(),
            new_description: Some(new_description),
            new_parent_ids: None,
        };
        self.rewrite_commit(rewrite)
    }

    /// Abandon a commit (its children will be rebased to its parents)
    pub fn abandon_commit(&self, commit_id: &FfiCommitId) -> Result<()> {
        self.with_transaction_mut(|tx| {
            // Convert commit ID
            let id = CommitId::try_from(commit_id).map_err(|e| JjError::InvalidArgument {
                message: format!("Invalid commit ID: {}", e),
            })?;

            // Get the commit
            let commit = tx
                .repo()
                .store()
                .get_commit(&id)
                .map_err(|e| JjError::Backend {
                    message: e.to_string(),
                })?;

            // Record as abandoned
            tx.repo_mut().record_abandoned_commit(&commit);

            Ok(())
        })
    }

    /// Commit the transaction and return the updated repository
    pub fn commit(&self, description: String) -> Result<Arc<FfiReadonlyRepo>> {
        let inner = self.take_transaction()?;

        let repo = inner.commit(&description).map_err(|e| JjError::Transaction {
            message: e.to_string(),
        })?;

        Ok(Arc::new(FfiReadonlyRepo::new(repo)))
    }

    /// Discard the transaction without committing
    pub fn discard(&self) -> Result<()> {
        let _ = self.take_transaction()?;
        // Transaction is dropped, no changes are committed
        Ok(())
    }

    /// Check if the transaction has any uncommitted changes
    pub fn has_changes(&self) -> Result<bool> {
        self.with_transaction(|tx| Ok(tx.repo().has_changes()))
    }
}
