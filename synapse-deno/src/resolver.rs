//! DenoResolver - Execute TypeScript/JavaScript resolvers

use crate::error::DenoError;
use crate::runtime::DenoRuntime;
use serde::{de::DeserializeOwned, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Configuration for the Deno resolver runtime
#[derive(Debug, Clone)]
pub struct DenoConfig {
    /// Base directory for resolving module paths
    pub base_dir: PathBuf,

    /// Default timeout for resolver execution in milliseconds
    pub default_timeout_ms: u32,

    /// Default permissions for resolver modules
    pub permissions: DenoPermissions,

    /// Whether to enable TypeScript type checking (slower but safer)
    pub type_check: bool,
}

impl Default for DenoConfig {
    fn default() -> Self {
        Self {
            base_dir: PathBuf::from("."),
            default_timeout_ms: 5000,
            permissions: DenoPermissions::default(),
            type_check: false, // Disable for performance, enable in development
        }
    }
}

/// Permissions for Deno resolver modules
#[derive(Debug, Clone, Default)]
pub struct DenoPermissions {
    /// Allowed network hosts (empty = no network access)
    pub net: Vec<String>,

    /// Allowed file system read paths (empty = no fs read access)
    pub read: Vec<String>,

    /// Allowed environment variables (empty = no env access)
    pub env: Vec<String>,
}

impl DenoPermissions {
    /// Create permissions that allow no external access (sandboxed)
    pub fn sandboxed() -> Self {
        Self::default()
    }

    /// Create permissions that allow read access to specific paths
    pub fn with_read(paths: Vec<String>) -> Self {
        Self {
            read: paths,
            ..Default::default()
        }
    }
}

/// Deno-based resolver executor
///
/// This struct manages a Deno runtime and provides methods for calling
/// TypeScript/JavaScript resolver functions from Rust.
pub struct DenoResolver {
    runtime: Arc<RwLock<DenoRuntime>>,
    config: DenoConfig,
}

impl DenoResolver {
    /// Create a new DenoResolver with the given configuration
    pub async fn new(config: DenoConfig) -> Result<Self, DenoError> {
        let runtime = DenoRuntime::new(&config)?;
        Ok(Self {
            runtime: Arc::new(RwLock::new(runtime)),
            config,
        })
    }

    /// Call a field resolver function
    ///
    /// # Arguments
    ///
    /// * `module` - Path to the TypeScript/JavaScript module (relative to base_dir)
    /// * `function` - Name of the exported resolver function
    /// * `parent` - The parent object (e.g., User)
    /// * `args` - Arguments passed to the resolver
    ///
    /// # Returns
    ///
    /// The result of the resolver function, deserialized to type `R`
    pub async fn call_field_resolver<P, A, R>(
        &self,
        module: &str,
        function: &str,
        parent: &P,
        args: &A,
    ) -> Result<R, DenoError>
    where
        P: Serialize,
        A: Serialize,
        R: DeserializeOwned,
    {
        let module_path = self.config.base_dir.join(module);

        // Serialize inputs to JSON
        let parent_json = serde_json::to_value(parent)?;
        let args_json = serde_json::to_value(args)?;

        // Execute in the runtime
        let mut runtime = self.runtime.write().await;
        let result = runtime
            .call_resolver(
                &module_path,
                function,
                parent_json,
                args_json,
                self.config.default_timeout_ms,
            )
            .await?;

        // Deserialize result
        let value: R = serde_json::from_value(result)?;
        Ok(value)
    }

    /// Call a root resolver function (for queries/mutations)
    ///
    /// # Arguments
    ///
    /// * `module` - Path to the TypeScript/JavaScript module
    /// * `function` - Name of the exported resolver function
    /// * `args` - Arguments passed to the resolver (typically the request)
    ///
    /// # Returns
    ///
    /// The result of the resolver function, deserialized to type `R`
    pub async fn call_root_resolver<A, R>(
        &self,
        module: &str,
        function: &str,
        args: &A,
    ) -> Result<R, DenoError>
    where
        A: Serialize,
        R: DeserializeOwned,
    {
        let module_path = self.config.base_dir.join(module);

        let args_json = serde_json::to_value(args)?;

        let mut runtime = self.runtime.write().await;
        let result = runtime
            .call_root_resolver(&module_path, function, args_json, self.config.default_timeout_ms)
            .await?;

        let value: R = serde_json::from_value(result)?;
        Ok(value)
    }

    /// Preload a module to warm up the cache
    pub async fn preload_module(&self, module: &str) -> Result<(), DenoError> {
        let module_path = self.config.base_dir.join(module);
        let mut runtime = self.runtime.write().await;
        runtime.load_module(&module_path).await
    }

    /// Get the configuration
    pub fn config(&self) -> &DenoConfig {
        &self.config
    }
}

impl std::fmt::Debug for DenoResolver {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DenoResolver")
            .field("config", &self.config)
            .finish()
    }
}
