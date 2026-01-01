//! Workspace operations for FFI

use std::path::Path;
use std::sync::{Arc, Mutex};

use jj_lib::config::{ConfigLayer, ConfigSource, StackedConfig};
use jj_lib::repo::{ReadonlyRepo, StoreFactories};
use jj_lib::settings::UserSettings;
use jj_lib::workspace::{default_working_copy_factories, Workspace};

use crate::error::{JjError, Result};
use crate::repo::FfiReadonlyRepo;

/// Create default user settings for FFI operations
fn create_user_settings(user_name: &str, user_email: &str) -> Result<UserSettings> {
    let mut config = StackedConfig::empty();

    // Create a config layer with user settings
    let toml_str = format!(
        r#"
[user]
name = "{}"
email = "{}"

[operation]
hostname = "ffi-client"
username = "ffi-user"
"#,
        user_name, user_email
    );

    let data: toml_edit::DocumentMut = toml_str.parse().map_err(|e| JjError::Internal {
        message: format!("Failed to parse config: {}", e),
    })?;

    let layer = ConfigLayer {
        source: ConfigSource::CommandArg,
        path: None,
        data,
    };
    config.add_layer(layer);

    UserSettings::from_config(config).map_err(|e| JjError::Internal {
        message: format!("Failed to create user settings: {}", e),
    })
}

/// A workspace exposed via FFI
#[derive(uniffi::Object)]
pub struct FfiWorkspace {
    inner: Mutex<Workspace>,
    repo: Arc<ReadonlyRepo>,
}

#[uniffi::export]
impl FfiWorkspace {
    /// Load an existing workspace from the given path
    #[uniffi::constructor]
    pub fn load(
        workspace_path: String,
        user_name: String,
        user_email: String,
    ) -> Result<Arc<Self>> {
        let path = Path::new(&workspace_path);
        let settings = create_user_settings(&user_name, &user_email)?;
        let store_factories = StoreFactories::default();
        let working_copy_factories = default_working_copy_factories();

        let workspace =
            Workspace::load(&settings, path, &store_factories, &working_copy_factories)?;

        let repo = workspace.repo_loader().load_at_head()?;

        Ok(Arc::new(Self {
            inner: Mutex::new(workspace),
            repo,
        }))
    }

    /// Get the workspace root path
    pub fn workspace_root(&self) -> String {
        let workspace = self.inner.lock().unwrap();
        workspace.workspace_root().to_string_lossy().to_string()
    }

    /// Get the repo path
    pub fn repo_path(&self) -> String {
        let workspace = self.inner.lock().unwrap();
        workspace.repo_path().to_string_lossy().to_string()
    }

    /// Get a readonly repository handle
    pub fn repo(&self) -> Arc<FfiReadonlyRepo> {
        Arc::new(FfiReadonlyRepo::new(Arc::clone(&self.repo)))
    }
}

/// Initialize a new Git workspace with internal Git backend
#[cfg(feature = "git")]
#[uniffi::export]
pub fn init_internal_git_workspace(
    workspace_path: String,
    user_name: String,
    user_email: String,
) -> Result<Arc<FfiWorkspace>> {
    let path = Path::new(&workspace_path);
    let settings = create_user_settings(&user_name, &user_email)?;

    let (workspace, repo) = Workspace::init_internal_git(&settings, path)?;

    Ok(Arc::new(FfiWorkspace {
        inner: Mutex::new(workspace),
        repo,
    }))
}

/// Initialize a new Git workspace with colocated Git backend
#[cfg(feature = "git")]
#[uniffi::export]
pub fn init_colocated_git_workspace(
    workspace_path: String,
    user_name: String,
    user_email: String,
) -> Result<Arc<FfiWorkspace>> {
    let path = Path::new(&workspace_path);
    let settings = create_user_settings(&user_name, &user_email)?;

    let (workspace, repo) = Workspace::init_colocated_git(&settings, path)?;

    Ok(Arc::new(FfiWorkspace {
        inner: Mutex::new(workspace),
        repo,
    }))
}
