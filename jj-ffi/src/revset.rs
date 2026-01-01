//! Revset operations for FFI

use std::collections::HashMap;
use std::sync::Arc;

use chrono::Local;
use jj_lib::repo::{ReadonlyRepo, Repo};
use jj_lib::revset::{
    RevsetAliasesMap, RevsetDiagnostics, RevsetExtensions, RevsetParseContext, SymbolResolver,
    parse,
};
use jj_lib::time_util::DatePatternContext;

use crate::error::{JjError, Result};
use crate::types::{FfiCommit, FfiCommitId};

/// Evaluate a revset expression and return matching commit IDs
pub fn evaluate_revset(
    repo: &Arc<ReadonlyRepo>,
    revset_str: &str,
    user_email: &str,
) -> Result<Vec<FfiCommitId>> {
    let aliases_map = RevsetAliasesMap::new();
    let extensions = RevsetExtensions::new();
    let date_context = DatePatternContext::from(Local::now());

    let context = RevsetParseContext {
        aliases_map: &aliases_map,
        local_variables: HashMap::new(),
        user_email,
        date_pattern_context: date_context,
        default_ignored_remote: None,
        use_glob_by_default: false,
        extensions: &extensions,
        workspace: None,
    };

    let mut diagnostics = RevsetDiagnostics::new();
    let user_expression = parse(&mut diagnostics, revset_str, &context).map_err(|e| {
        JjError::Revset {
            message: e.to_string(),
        }
    })?;

    let symbol_resolver = SymbolResolver::new(repo.as_ref(), extensions.symbol_resolvers());
    let resolved_expression = user_expression
        .resolve_user_expression(repo.as_ref(), &symbol_resolver)
        .map_err(|e| JjError::Revset {
            message: e.to_string(),
        })?;

    let revset = resolved_expression.evaluate(repo.as_ref()).map_err(|e| {
        JjError::Revset {
            message: e.to_string(),
        }
    })?;

    let mut commit_ids = Vec::new();
    for result in revset.iter() {
        let commit_id = result.map_err(|e| JjError::Revset {
            message: e.to_string(),
        })?;
        commit_ids.push(FfiCommitId::from(&commit_id));
    }

    Ok(commit_ids)
}

/// Evaluate a revset expression and return matching commits
pub fn evaluate_revset_to_commits(
    repo: &Arc<ReadonlyRepo>,
    revset_str: &str,
    user_email: &str,
) -> Result<Vec<FfiCommit>> {
    use jj_lib::revset::RevsetIteratorExt;

    let aliases_map = RevsetAliasesMap::new();
    let extensions = RevsetExtensions::new();
    let date_context = DatePatternContext::from(Local::now());

    let context = RevsetParseContext {
        aliases_map: &aliases_map,
        local_variables: HashMap::new(),
        user_email,
        date_pattern_context: date_context,
        default_ignored_remote: None,
        use_glob_by_default: false,
        extensions: &extensions,
        workspace: None,
    };

    let mut diagnostics = RevsetDiagnostics::new();
    let user_expression = parse(&mut diagnostics, revset_str, &context).map_err(|e| {
        JjError::Revset {
            message: e.to_string(),
        }
    })?;

    let symbol_resolver = SymbolResolver::new(repo.as_ref(), extensions.symbol_resolvers());
    let resolved_expression = user_expression
        .resolve_user_expression(repo.as_ref(), &symbol_resolver)
        .map_err(|e| JjError::Revset {
            message: e.to_string(),
        })?;

    let revset = resolved_expression.evaluate(repo.as_ref()).map_err(|e| {
        JjError::Revset {
            message: e.to_string(),
        }
    })?;

    let store = repo.store();
    let mut commits = Vec::new();
    for result in revset.iter().commits(store) {
        let commit = result.map_err(|e| JjError::Revset {
            message: e.to_string(),
        })?;
        commits.push(FfiCommit::from(&commit));
    }

    Ok(commits)
}

/// Count commits matching a revset expression
pub fn count_revset(repo: &Arc<ReadonlyRepo>, revset_str: &str, user_email: &str) -> Result<u64> {
    let aliases_map = RevsetAliasesMap::new();
    let extensions = RevsetExtensions::new();
    let date_context = DatePatternContext::from(Local::now());

    let context = RevsetParseContext {
        aliases_map: &aliases_map,
        local_variables: HashMap::new(),
        user_email,
        date_pattern_context: date_context,
        default_ignored_remote: None,
        use_glob_by_default: false,
        extensions: &extensions,
        workspace: None,
    };

    let mut diagnostics = RevsetDiagnostics::new();
    let user_expression = parse(&mut diagnostics, revset_str, &context).map_err(|e| {
        JjError::Revset {
            message: e.to_string(),
        }
    })?;

    let symbol_resolver = SymbolResolver::new(repo.as_ref(), extensions.symbol_resolvers());
    let resolved_expression = user_expression
        .resolve_user_expression(repo.as_ref(), &symbol_resolver)
        .map_err(|e| JjError::Revset {
            message: e.to_string(),
        })?;

    let revset = resolved_expression.evaluate(repo.as_ref()).map_err(|e| {
        JjError::Revset {
            message: e.to_string(),
        }
    })?;

    let mut count = 0u64;
    for result in revset.iter() {
        result.map_err(|e| JjError::Revset {
            message: e.to_string(),
        })?;
        count += 1;
    }

    Ok(count)
}
