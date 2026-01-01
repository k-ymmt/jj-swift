//! Git operations for FFI

use std::sync::{Arc, Mutex};

use jj_lib::git::{
    self, GitFetch, GitImportStats, GitSettings, RemoteCallbacks,
    expand_fetch_refspecs,
};
use jj_lib::ref_name::{RefName, RemoteName};
use jj_lib::repo::Repo;
use jj_lib::str_util::{StringExpression, StringPattern};
use jj_lib::transaction::Transaction;

use crate::error::{JjError, Result};
use crate::repo::FfiReadonlyRepo;
use crate::types::FfiCommitId;

/// Statistics from a git import operation
#[derive(Debug, Clone, uniffi::Record)]
pub struct FfiGitImportStats {
    /// Number of commits that were abandoned
    pub abandoned_commits_count: u64,
    /// Number of remote bookmarks that changed
    pub changed_remote_bookmarks_count: u64,
    /// Number of remote tags that changed
    pub changed_remote_tags_count: u64,
    /// Number of refs that failed to import
    pub failed_refs_count: u64,
}

impl From<&GitImportStats> for FfiGitImportStats {
    fn from(stats: &GitImportStats) -> Self {
        Self {
            abandoned_commits_count: stats.abandoned_commits.len() as u64,
            changed_remote_bookmarks_count: stats.changed_remote_bookmarks.len() as u64,
            changed_remote_tags_count: stats.changed_remote_tags.len() as u64,
            failed_refs_count: stats.failed_ref_names.len() as u64,
        }
    }
}

/// Statistics from a git export operation
#[derive(Debug, Clone, uniffi::Record)]
pub struct FfiGitExportStats {
    /// Number of bookmarks that failed to export
    pub failed_bookmarks_count: u64,
    /// Number of tags that failed to export
    pub failed_tags_count: u64,
}

/// Statistics from a git push operation
#[derive(Debug, Clone, uniffi::Record)]
pub struct FfiGitPushStats {
    /// Number of refs that were successfully pushed
    pub pushed_count: u64,
    /// Number of refs that were rejected (lease failure)
    pub rejected_count: u64,
    /// Number of refs that were rejected by the remote
    pub remote_rejected_count: u64,
    /// Whether all refs were pushed successfully
    pub all_ok: bool,
}

/// A Git transaction for performing Git operations
///
/// This wraps a jj Transaction and provides Git-specific operations.
#[derive(uniffi::Object)]
pub struct FfiGitTransaction {
    inner: Mutex<Option<Transaction>>,
    git_settings: GitSettings,
}

// SAFETY: FfiGitTransaction is protected by a Mutex, ensuring synchronized access
unsafe impl Send for FfiGitTransaction {}
unsafe impl Sync for FfiGitTransaction {}

impl FfiGitTransaction {
    pub(crate) fn new(transaction: Transaction, git_settings: GitSettings) -> Self {
        Self {
            inner: Mutex::new(Some(transaction)),
            git_settings,
        }
    }

    fn with_transaction_mut<T, F>(&self, f: F) -> Result<T>
    where
        F: FnOnce(&mut Transaction, &GitSettings) -> Result<T>,
    {
        let mut guard = self.inner.lock().map_err(|_| JjError::Internal {
            message: "Failed to acquire transaction lock".to_string(),
        })?;
        let tx = guard.as_mut().ok_or_else(|| JjError::Internal {
            message: "Transaction has already been committed or discarded".to_string(),
        })?;
        f(tx, &self.git_settings)
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
impl FfiGitTransaction {
    /// Import refs from the underlying Git repository
    ///
    /// This synchronizes the jj view with any changes made directly in Git.
    pub fn import_refs(&self) -> Result<FfiGitImportStats> {
        self.with_transaction_mut(|tx, git_settings| {
            let stats = git::import_refs(tx.repo_mut(), git_settings).map_err(|e| JjError::Git {
                message: e.to_string(),
            })?;
            Ok(FfiGitImportStats::from(&stats))
        })
    }

    /// Export refs to the underlying Git repository
    ///
    /// This synchronizes the Git refs with changes made in jj.
    pub fn export_refs(&self) -> Result<FfiGitExportStats> {
        self.with_transaction_mut(|tx, _git_settings| {
            let stats = git::export_refs(tx.repo_mut()).map_err(|e| JjError::Git {
                message: e.to_string(),
            })?;
            Ok(FfiGitExportStats {
                failed_bookmarks_count: stats.failed_bookmarks.len() as u64,
                failed_tags_count: stats.failed_tags.len() as u64,
            })
        })
    }

    /// Fetch from a remote
    ///
    /// Fetches the specified branches (or all branches if empty) from the remote.
    pub fn fetch(
        &self,
        remote_name: String,
        branch_patterns: Vec<String>,
    ) -> Result<FfiGitImportStats> {
        self.with_transaction_mut(|tx, git_settings| {
            let remote = RemoteName::new(&remote_name);

            // Create GitFetch helper
            let mut git_fetch =
                GitFetch::new(tx.repo_mut(), git_settings).map_err(|e| JjError::Git {
                    message: e.to_string(),
                })?;

            // Build branch expression
            let branch_expr = if branch_patterns.is_empty() {
                StringExpression::all()
            } else {
                let expressions: Vec<StringExpression> = branch_patterns
                    .iter()
                    .map(|p| {
                        if p.contains('*') {
                            // Parse as glob pattern
                            match StringPattern::glob(p) {
                                Ok(pattern) => StringExpression::pattern(pattern),
                                Err(_) => StringExpression::exact(p.clone()),
                            }
                        } else {
                            StringExpression::exact(p.clone())
                        }
                    })
                    .collect();
                StringExpression::union_all(expressions)
            };

            // Expand refspecs
            let refspecs = expand_fetch_refspecs(remote, branch_expr).map_err(|e| {
                JjError::Git {
                    message: e.to_string(),
                }
            })?;

            // Perform fetch
            let callbacks = RemoteCallbacks::default();
            git_fetch
                .fetch(remote, refspecs, callbacks, None, None)
                .map_err(|e| JjError::Git {
                    message: e.to_string(),
                })?;

            // Import the fetched refs
            let stats = git_fetch.import_refs().map_err(|e| JjError::Git {
                message: e.to_string(),
            })?;

            Ok(FfiGitImportStats::from(&stats))
        })
    }

    /// Push branches to a remote
    ///
    /// Pushes the specified local branches to the remote.
    pub fn push_branches(
        &self,
        remote_name: String,
        branch_names: Vec<String>,
    ) -> Result<FfiGitPushStats> {
        self.with_transaction_mut(|tx, git_settings| {
            let remote = RemoteName::new(&remote_name);

            // Build the push targets from branch names
            let mut branch_updates = Vec::new();
            let view = tx.repo().view();

            for branch_name in &branch_names {
                let ref_name = RefName::new(branch_name);
                let local_target = view.get_local_bookmark(ref_name);
                if local_target.is_absent() {
                    return Err(JjError::Git {
                        message: format!("Branch '{}' not found", branch_name),
                    });
                }

                // Get the remote tracking branch's current target (if any)
                let symbol = ref_name.to_remote_symbol(remote);
                let remote_ref = view.get_remote_bookmark(symbol);
                let old_target = remote_ref.target.as_normal().cloned();
                let new_target = local_target.as_normal().cloned();

                branch_updates.push((
                    branch_name.as_str().into(),
                    jj_lib::refs::BookmarkPushUpdate {
                        old_target,
                        new_target,
                    },
                ));
            }

            let targets = git::GitBranchPushTargets { branch_updates };
            let callbacks = RemoteCallbacks::default();

            let stats = git::push_branches(tx.repo_mut(), git_settings, remote, &targets, callbacks)
                .map_err(|e| JjError::Git {
                    message: e.to_string(),
                })?;

            Ok(FfiGitPushStats {
                pushed_count: stats.pushed.len() as u64,
                rejected_count: stats.rejected.len() as u64,
                remote_rejected_count: stats.remote_rejected.len() as u64,
                all_ok: stats.all_ok(),
            })
        })
    }

    /// Commit the git transaction and return the updated repository
    pub fn commit(&self, description: String) -> Result<Arc<FfiReadonlyRepo>> {
        let inner = self.take_transaction()?;

        let repo = inner.commit(&description).map_err(|e| JjError::Transaction {
            message: e.to_string(),
        })?;

        Ok(Arc::new(FfiReadonlyRepo::new(repo)))
    }

    /// Discard the git transaction without committing
    pub fn discard(&self) -> Result<()> {
        let _ = self.take_transaction()?;
        Ok(())
    }
}

/// Get abandoned commit IDs from import stats
#[uniffi::export]
pub fn get_abandoned_commits_from_import(_stats: &FfiGitImportStats) -> Vec<FfiCommitId> {
    // Note: This is a simplified version - the actual abandoned commits would need
    // to be stored separately if needed
    Vec::new()
}
