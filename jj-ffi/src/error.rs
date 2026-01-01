//! Unified error type for jj-ffi

use jj_lib::backend::BackendError;
use jj_lib::repo::{RepoLoaderError, StoreLoadError};
use jj_lib::transaction::TransactionCommitError;
use jj_lib::workspace::{WorkspaceInitError, WorkspaceLoadError};

/// Unified error type exposed via FFI
#[derive(Debug, thiserror::Error, uniffi::Error)]
pub enum JjError {
    #[error("Workspace error: {message}")]
    Workspace { message: String },

    #[error("Repository error: {message}")]
    Repository { message: String },

    #[error("Backend error: {message}")]
    Backend { message: String },

    #[error("Commit not found: {id}")]
    CommitNotFound { id: String },

    #[error("Invalid argument: {message}")]
    InvalidArgument { message: String },

    #[error("Revset error: {message}")]
    Revset { message: String },

    #[error("Transaction error: {message}")]
    Transaction { message: String },

    #[error("Git error: {message}")]
    Git { message: String },

    #[error("Internal error: {message}")]
    Internal { message: String },
}

impl From<WorkspaceLoadError> for JjError {
    fn from(err: WorkspaceLoadError) -> Self {
        JjError::Workspace {
            message: err.to_string(),
        }
    }
}

impl From<WorkspaceInitError> for JjError {
    fn from(err: WorkspaceInitError) -> Self {
        JjError::Workspace {
            message: err.to_string(),
        }
    }
}

impl From<StoreLoadError> for JjError {
    fn from(err: StoreLoadError) -> Self {
        JjError::Repository {
            message: err.to_string(),
        }
    }
}

impl From<BackendError> for JjError {
    fn from(err: BackendError) -> Self {
        JjError::Backend {
            message: err.to_string(),
        }
    }
}

impl From<RepoLoaderError> for JjError {
    fn from(err: RepoLoaderError) -> Self {
        JjError::Repository {
            message: err.to_string(),
        }
    }
}

impl From<TransactionCommitError> for JjError {
    fn from(err: TransactionCommitError) -> Self {
        JjError::Transaction {
            message: err.to_string(),
        }
    }
}

pub type Result<T> = std::result::Result<T, JjError>;
