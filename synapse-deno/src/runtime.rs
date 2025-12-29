//! Deno runtime wrapper

use crate::error::DenoError;
use crate::resolver::DenoConfig;
use deno_core::{extension, op2, JsRuntime, ModuleSpecifier, PollEventLoopOptions, RuntimeOptions};
use serde_json::Value;
use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex};

/// Shared storage for results between JS and Rust
type SharedResult = Arc<Mutex<Option<Value>>>;

thread_local! {
    static RESULT_STORAGE: SharedResult = Arc::new(Mutex::new(None));
}

/// Internal Deno runtime wrapper
pub struct DenoRuntime {
    runtime: JsRuntime,
    /// Cache of loaded module IDs
    loaded_modules: HashMap<String, deno_core::ModuleId>,
}

// Extension for synapse resolver operations
extension!(
    synapse_resolver,
    ops = [op_log, op_return_result],
);

/// Log operation for debugging from JS
#[op2(fast)]
fn op_log(#[string] msg: &str) {
    tracing::debug!(target: "synapse_deno", "{}", msg);
}

/// Return a result from JS to Rust
#[op2]
fn op_return_result(#[serde] value: serde_json::Value) {
    RESULT_STORAGE.with(|storage| {
        *storage.lock().unwrap() = Some(value);
    });
}

/// Get the stored result
fn take_result() -> Option<Value> {
    RESULT_STORAGE.with(|storage| storage.lock().unwrap().take())
}

impl DenoRuntime {
    /// Create a new Deno runtime
    pub fn new(_config: &DenoConfig) -> Result<Self, DenoError> {
        let runtime = JsRuntime::new(RuntimeOptions {
            extensions: vec![synapse_resolver::init()],
            ..Default::default()
        });

        Ok(Self {
            runtime,
            loaded_modules: HashMap::new(),
        })
    }

    /// Load a module from disk
    pub async fn load_module(&mut self, path: &Path) -> Result<(), DenoError> {
        let path_str = path.to_string_lossy().to_string();

        if self.loaded_modules.contains_key(&path_str) {
            return Ok(());
        }

        let specifier = ModuleSpecifier::from_file_path(path).map_err(|_| {
            DenoError::ModuleLoadError {
                module: path_str.clone(),
                reason: "Invalid file path".to_string(),
            }
        })?;

        let module_id = self
            .runtime
            .load_side_es_module(&specifier)
            .await
            .map_err(|e| DenoError::ModuleLoadError {
                module: path_str.clone(),
                reason: e.to_string(),
            })?;

        // Evaluate the module
        let result = self.runtime.mod_evaluate(module_id);

        // Run the event loop to completion
        self.runtime
            .run_event_loop(PollEventLoopOptions::default())
            .await
            .map_err(|e| DenoError::ModuleLoadError {
                module: path_str.clone(),
                reason: e.to_string(),
            })?;

        // Wait for module evaluation
        result.await.map_err(|e| DenoError::ModuleLoadError {
            module: path_str.clone(),
            reason: e.to_string(),
        })?;

        self.loaded_modules.insert(path_str, module_id);
        Ok(())
    }

    /// Call a field resolver function
    pub async fn call_resolver(
        &mut self,
        module_path: &Path,
        function: &str,
        parent: Value,
        args: Value,
        _timeout_ms: u32,
    ) -> Result<Value, DenoError> {
        // Ensure module is loaded
        self.load_module(module_path).await?;

        let path_str = module_path.to_string_lossy().to_string();

        // Build the call script (execute_script will wrap this in async/await)
        let script = format!(
            r#"
            (async () => {{
                const module = await import("file://{}");
                const fn = module["{}"];
                if (typeof fn !== "function") {{
                    throw new Error("Function '{}' not found in module");
                }}
                const parent = {};
                const args = {};
                const ctx = globalThis.__synapse_context || {{}};
                return fn(parent, args, ctx);
            }})()
            "#,
            path_str,
            function,
            function,
            serde_json::to_string(&parent).unwrap_or_default(),
            serde_json::to_string(&args).unwrap_or_default(),
        );

        self.execute_script(&script, &path_str, function).await
    }

    /// Call a root resolver function (no parent)
    pub async fn call_root_resolver(
        &mut self,
        module_path: &Path,
        function: &str,
        args: Value,
        _timeout_ms: u32,
    ) -> Result<Value, DenoError> {
        self.load_module(module_path).await?;

        let path_str = module_path.to_string_lossy().to_string();

        let script = format!(
            r#"
            (async () => {{
                const module = await import("file://{}");
                const fn = module["{}"];
                if (typeof fn !== "function") {{
                    throw new Error("Function '{}' not found in module");
                }}
                const args = {};
                const ctx = globalThis.__synapse_context || {{}};
                return await fn(args, ctx);
            }})()
            "#,
            path_str,
            function,
            function,
            serde_json::to_string(&args).unwrap_or_default(),
        );

        self.execute_script(&script, &path_str, function).await
    }

    /// Execute a script and return the result
    async fn execute_script(
        &mut self,
        script: &str,
        module: &str,
        function: &str,
    ) -> Result<Value, DenoError> {
        // Clear any previous result
        take_result();

        // Wrap script to call op_return_result with the result
        let wrapped_script = format!(
            r#"
            (async () => {{
                const result = await ({});
                Deno.core.ops.op_return_result(result);
            }})()
            "#,
            script
        );

        // Execute the script
        let promise = self
            .runtime
            .execute_script("<resolver>", wrapped_script)
            .map_err(|e| DenoError::ExecutionError {
                module: module.to_string(),
                function: function.to_string(),
                reason: e.to_string(),
            })?;

        // Run event loop to complete the promise
        self.runtime
            .run_event_loop(PollEventLoopOptions::default())
            .await
            .map_err(|e| DenoError::ExecutionError {
                module: module.to_string(),
                function: function.to_string(),
                reason: e.to_string(),
            })?;

        // Resolve the promise
        let _ = self.runtime.resolve(promise).await;

        // Get the result from thread-local storage
        take_result().ok_or_else(|| DenoError::ExecutionError {
            module: module.to_string(),
            function: function.to_string(),
            reason: "No result returned from resolver".to_string(),
        })
    }
}
