//! Log operations for FFI
//!
//! This module provides log functionality similar to `jj log` command,
//! exposing graph-based commit history via FFI.

use std::collections::HashMap;
use std::sync::Arc;

use chrono::Local;
use jj_lib::backend::CommitId;
use jj_lib::graph::{GraphEdge, GraphEdgeType, TopoGroupedGraphIterator, reverse_graph};
use jj_lib::repo::{ReadonlyRepo, Repo};
use jj_lib::revset::{
    RevsetAliasesMap, RevsetDiagnostics, RevsetExpression, RevsetExtensions, RevsetIteratorExt,
    RevsetParseContext, SymbolResolver, parse,
};
use jj_lib::time_util::DatePatternContext;

use crate::error::{JjError, Result};
use crate::types::{FfiCommit, FfiCommitId};

/// Graph edge type exposed via FFI
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum FfiGraphEdgeType {
    /// Direct parent-child relationship
    Direct,
    /// Indirect relationship (some commits in between are elided)
    Indirect,
    /// Missing parent (incomplete history)
    Missing,
}

impl From<GraphEdgeType> for FfiGraphEdgeType {
    fn from(edge_type: GraphEdgeType) -> Self {
        match edge_type {
            GraphEdgeType::Direct => FfiGraphEdgeType::Direct,
            GraphEdgeType::Indirect => FfiGraphEdgeType::Indirect,
            GraphEdgeType::Missing => FfiGraphEdgeType::Missing,
        }
    }
}

/// A graph edge connecting to a parent commit
#[derive(Debug, Clone, uniffi::Record)]
pub struct FfiGraphEdge {
    /// The target commit ID (parent)
    pub target: FfiCommitId,
    /// The type of edge
    pub edge_type: FfiGraphEdgeType,
}

impl FfiGraphEdge {
    fn from_graph_edge(edge: &GraphEdge<CommitId>) -> Self {
        Self {
            target: FfiCommitId::from(&edge.target),
            edge_type: FfiGraphEdgeType::from(edge.edge_type),
        }
    }
}

/// A log entry containing commit information and graph edges
#[derive(Debug, Clone, uniffi::Record)]
pub struct FfiLogEntry {
    /// The commit information
    pub commit: FfiCommit,
    /// Edges to parent commits in the graph
    pub edges: Vec<FfiGraphEdge>,
}

/// Options for log retrieval
#[derive(Debug, Clone, Default, uniffi::Record)]
pub struct FfiLogOptions {
    /// Revset expressions to evaluate (if empty, uses default)
    pub revisions: Vec<String>,
    /// Maximum number of commits to return (-1 for no limit)
    pub limit: i64,
    /// Whether to return commits in reverse order (oldest first)
    pub reversed: bool,
}

/// Result of a log operation
#[derive(Debug, Clone, uniffi::Record)]
pub struct FfiLogResult {
    /// Log entries with graph information
    pub entries: Vec<FfiLogEntry>,
}

/// Evaluate log with graph information
pub fn evaluate_log(
    repo: &Arc<ReadonlyRepo>,
    options: &FfiLogOptions,
    user_email: &str,
) -> Result<FfiLogResult> {
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

    // Build revset expression
    let revset_expression = if options.revisions.is_empty() {
        // Default: all visible commits
        RevsetExpression::all()
    } else {
        // Union of all provided revisions
        let mut expressions = Vec::new();
        let mut diagnostics = RevsetDiagnostics::new();
        for rev_str in &options.revisions {
            let expr = parse(&mut diagnostics, rev_str, &context).map_err(|e| JjError::Revset {
                message: e.to_string(),
            })?;
            expressions.push(expr);
        }
        // Union all expressions
        expressions
            .into_iter()
            .reduce(|a, b| a.union(&b))
            .unwrap_or_else(RevsetExpression::none)
    };

    // Resolve and evaluate
    let symbol_resolver = SymbolResolver::new(repo.as_ref(), extensions.symbol_resolvers());
    let resolved_expression = revset_expression
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
    let limit = if options.limit < 0 {
        usize::MAX
    } else {
        options.limit as usize
    };

    // Use TopoGroupedGraphIterator for proper graph ordering
    let graph_iter = TopoGroupedGraphIterator::new(revset.iter_graph(), |id| id);
    let graph_iter = graph_iter.take(limit);

    let entries: Vec<FfiLogEntry> = if options.reversed {
        // Reverse the graph
        let reversed = reverse_graph(graph_iter, |id| id).map_err(|e| JjError::Revset {
            message: e.to_string(),
        })?;

        reversed
            .into_iter()
            .map(|(commit_id, edges)| {
                let commit = store.get_commit(&commit_id)?;
                Ok(FfiLogEntry {
                    commit: FfiCommit::from(&commit),
                    edges: edges.iter().map(FfiGraphEdge::from_graph_edge).collect(),
                })
            })
            .collect::<Result<Vec<_>>>()?
    } else {
        graph_iter
            .map(|result| {
                let (commit_id, edges) = result.map_err(|e| JjError::Revset {
                    message: e.to_string(),
                })?;
                let commit = store.get_commit(&commit_id)?;
                Ok(FfiLogEntry {
                    commit: FfiCommit::from(&commit),
                    edges: edges.iter().map(FfiGraphEdge::from_graph_edge).collect(),
                })
            })
            .collect::<Result<Vec<_>>>()?
    };

    Ok(FfiLogResult { entries })
}

/// Evaluate log without graph information (flat list)
pub fn evaluate_log_flat(
    repo: &Arc<ReadonlyRepo>,
    options: &FfiLogOptions,
    user_email: &str,
) -> Result<Vec<FfiCommit>> {
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

    // Build revset expression
    let revset_expression = if options.revisions.is_empty() {
        RevsetExpression::all()
    } else {
        let mut expressions = Vec::new();
        let mut diagnostics = RevsetDiagnostics::new();
        for rev_str in &options.revisions {
            let expr = parse(&mut diagnostics, rev_str, &context).map_err(|e| JjError::Revset {
                message: e.to_string(),
            })?;
            expressions.push(expr);
        }
        expressions
            .into_iter()
            .reduce(|a, b| a.union(&b))
            .unwrap_or_else(RevsetExpression::none)
    };

    // Resolve and evaluate
    let symbol_resolver = SymbolResolver::new(repo.as_ref(), extensions.symbol_resolvers());
    let resolved_expression = revset_expression
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
    let limit = if options.limit < 0 {
        usize::MAX
    } else {
        options.limit as usize
    };

    let iter = revset.iter().take(limit);

    let commits: Vec<FfiCommit> = if options.reversed {
        let commit_ids: Vec<CommitId> = iter
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(|e| JjError::Revset {
                message: e.to_string(),
            })?;

        commit_ids
            .into_iter()
            .rev()
            .map(|id| {
                let commit = store.get_commit(&id)?;
                Ok(FfiCommit::from(&commit))
            })
            .collect::<Result<Vec<_>>>()?
    } else {
        iter.commits(store)
            .map(|result| {
                let commit = result.map_err(|e| JjError::Revset {
                    message: e.to_string(),
                })?;
                Ok(FfiCommit::from(&commit))
            })
            .collect::<Result<Vec<_>>>()?
    };

    Ok(commits)
}
