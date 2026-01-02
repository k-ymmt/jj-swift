//! jj-ffi: UniFFI bindings for jj-lib
//!
//! This crate provides FFI bindings to expose jj-lib functionality
//! to other languages via UniFFI.

pub mod error;
#[cfg(feature = "git")]
pub mod git;
pub mod log;
pub mod repo;
pub mod revset;
pub mod transaction;
pub mod types;
pub mod workspace;

// Re-export main types for convenience
pub use error::JjError;
pub use log::{FfiGraphEdge, FfiGraphEdgeType, FfiLogEntry, FfiLogOptions, FfiLogResult};
pub use repo::FfiReadonlyRepo;
pub use transaction::FfiTransaction;
pub use types::{
    FfiChangeId, FfiCommit, FfiCommitId, FfiNewCommit, FfiRewriteCommit, FfiSignature, FfiTimestamp,
};
pub use workspace::FfiWorkspace;

#[cfg(feature = "git")]
pub use git::{FfiGitExportStats, FfiGitImportStats, FfiGitPushStats, FfiGitTransaction};
#[cfg(feature = "git")]
pub use workspace::{init_colocated_git_workspace, init_internal_git_workspace};

// UniFFI scaffolding
uniffi::setup_scaffolding!();
