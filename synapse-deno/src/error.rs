//! Error types for Deno runtime operations

use thiserror::Error;

/// Errors that can occur during Deno runtime operations
#[derive(Error, Debug)]
pub enum DenoError {
    /// Failed to initialize the Deno runtime
    #[error("Failed to initialize Deno runtime: {0}")]
    InitError(String),

    /// Failed to load a module
    #[error("Failed to load module '{module}': {reason}")]
    ModuleLoadError {
        /// The module path that failed to load
        module: String,
        /// The reason for the failure
        reason: String,
    },

    /// Failed to execute a resolver function
    #[error("Failed to execute resolver '{function}' in '{module}': {reason}")]
    ExecutionError {
        /// The module containing the function
        module: String,
        /// The function that failed
        function: String,
        /// The reason for the failure
        reason: String,
    },

    /// Failed to serialize/deserialize data
    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    /// JavaScript error from the runtime
    #[error("JavaScript error: {0}")]
    JsError(String),

    /// Module not found
    #[error("Module not found: {0}")]
    ModuleNotFound(String),

    /// Function not exported from module
    #[error("Function '{function}' not exported from module '{module}'")]
    FunctionNotExported {
        /// The module path
        module: String,
        /// The function name
        function: String,
    },

    /// Timeout during execution
    #[error("Execution timeout after {0}ms")]
    Timeout(u32),

    /// Permission denied
    #[error("Permission denied: {0}")]
    PermissionDenied(String),
}
