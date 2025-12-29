//! Simple integration test for DenoResolver

use std::path::PathBuf;
use synapse_deno::{DenoConfig, DenoResolver};

fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

#[tokio::test(flavor = "current_thread")]
async fn test_simple_resolver() {
    let config = DenoConfig {
        base_dir: fixtures_dir(),
        ..Default::default()
    };

    let resolver = DenoResolver::new(config).await.expect("Failed to create resolver");

    // Test with a simple object
    let user = serde_json::json!({
        "id": 1,
        "name": "John Doe",
        "email": "john@example.com"
    });

    let result: String = resolver
        .call_field_resolver("user_resolver.js", "displayName", &user, &serde_json::json!({}))
        .await
        .expect("Failed to call resolver");

    assert_eq!(result, "John Doe");
}
