//! Deno runtime wrapper

use crate::error::DenoError;
use crate::resolver::DenoConfig;
use deno_core::{
    extension, op2, JsRuntime, ModuleLoadResponse, ModuleLoader, ModuleSource, ModuleSourceCode,
    ModuleSpecifier, ModuleType, PollEventLoopOptions, ResolutionKind, RuntimeOptions,
};
use deno_error::JsErrorBox;
use serde_json::Value;
use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex};

/// Shared storage for results between JS and Rust
type SharedResult = Arc<Mutex<Option<Value>>>;

thread_local! {
    static RESULT_STORAGE: SharedResult = Arc::new(Mutex::new(None));
}

/// Simple file-based module loader
struct FileModuleLoader;

impl ModuleLoader for FileModuleLoader {
    fn resolve(
        &self,
        specifier: &str,
        referrer: &str,
        _kind: ResolutionKind,
    ) -> Result<ModuleSpecifier, JsErrorBox> {
        // Handle file:// URLs
        if specifier.starts_with("file://") {
            return ModuleSpecifier::parse(specifier)
                .map_err(|e| JsErrorBox::generic(e.to_string()));
        }

        // Handle relative imports
        if specifier.starts_with("./") || specifier.starts_with("../") {
            let referrer_url = ModuleSpecifier::parse(referrer)
                .map_err(|e| JsErrorBox::generic(e.to_string()))?;
            return referrer_url
                .join(specifier)
                .map_err(|e| JsErrorBox::generic(e.to_string()));
        }

        // Try to parse as absolute URL
        if let Ok(url) = ModuleSpecifier::parse(specifier) {
            return Ok(url);
        }

        Err(JsErrorBox::generic(format!(
            "Cannot resolve module: {}",
            specifier
        )))
    }

    fn load(
        &self,
        module_specifier: &ModuleSpecifier,
        _maybe_referrer: Option<&deno_core::ModuleLoadReferrer>,
        _options: deno_core::ModuleLoadOptions,
    ) -> ModuleLoadResponse {
        let specifier = module_specifier.clone();

        ModuleLoadResponse::Sync(load_module_sync(&specifier))
    }
}

/// Load a module synchronously from the filesystem
fn load_module_sync(specifier: &ModuleSpecifier) -> Result<ModuleSource, JsErrorBox> {
    if specifier.scheme() != "file" {
        return Err(JsErrorBox::generic(format!(
            "Only file:// URLs are supported, got: {}",
            specifier
        )));
    }

    let path = specifier
        .to_file_path()
        .map_err(|_| JsErrorBox::generic("Invalid file path"))?;

    let code =
        std::fs::read_to_string(&path).map_err(|e| JsErrorBox::generic(e.to_string()))?;

    // Determine module type based on extension
    let module_type = if path.extension().map_or(false, |ext| ext == "json") {
        ModuleType::Json
    } else {
        ModuleType::JavaScript
    };

    Ok(ModuleSource::new(
        module_type,
        ModuleSourceCode::String(code.into()),
        specifier,
        None,
    ))
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
            module_loader: Some(std::rc::Rc::new(FileModuleLoader)),
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
        let path_str = module_path.to_string_lossy().to_string();

        // Build the call script - use dynamic import
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
                return await fn(parent, args, ctx);
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
                try {{
                    const result = await ({});
                    Deno.core.ops.op_return_result(result);
                }} catch (e) {{
                    Deno.core.ops.op_return_result({{ __error: e.message || String(e) }});
                }}
            }})()
            "#,
            script
        );

        // Execute the script - this starts the async operation
        let promise = self
            .runtime
            .execute_script("<resolver>", wrapped_script)
            .map_err(|e| DenoError::ExecutionError {
                module: module.to_string(),
                function: function.to_string(),
                reason: e.to_string(),
            })?;

        // We need to drive the event loop AND resolve the promise concurrently
        // The resolve() call will complete when the promise settles
        let resolve_future = self.runtime.resolve(promise);

        // Run the event loop until the promise resolves
        tokio::select! {
            result = resolve_future => {
                result.map_err(|e| DenoError::ExecutionError {
                    module: module.to_string(),
                    function: function.to_string(),
                    reason: e.to_string(),
                })?;
            }
            result = self.runtime.run_event_loop(PollEventLoopOptions::default()) => {
                result.map_err(|e| DenoError::ExecutionError {
                    module: module.to_string(),
                    function: function.to_string(),
                    reason: e.to_string(),
                })?;
            }
        }

        // Get the result from thread-local storage
        let result = take_result().ok_or_else(|| DenoError::ExecutionError {
            module: module.to_string(),
            function: function.to_string(),
            reason: "No result returned from resolver".to_string(),
        })?;

        // Check if the result is an error
        if let Some(err) = result.get("__error") {
            return Err(DenoError::JsError(err.as_str().unwrap_or("Unknown error").to_string()));
        }

        Ok(result)
    }
}
