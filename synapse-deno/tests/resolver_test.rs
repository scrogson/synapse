//! Integration tests for DenoResolver

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use synapse_deno::{DenoConfig, DenoResolver};

/// Test user struct
#[derive(Debug, Serialize, Deserialize)]
struct User {
    id: i64,
    name: String,
    email: String,
    #[serde(rename = "firstName")]
    first_name: Option<String>,
    #[serde(rename = "lastName")]
    last_name: Option<String>,
}

/// Empty args for resolvers that don't need arguments
#[derive(Debug, Serialize, Deserialize)]
struct EmptyArgs {}

/// Args for the greeting resolver
#[derive(Debug, Serialize, Deserialize)]
struct GreetingArgs {
    name: Option<String>,
}

fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

#[tokio::test]
async fn test_display_name_resolver() {
    let config = DenoConfig {
        base_dir: fixtures_dir(),
        ..Default::default()
    };

    let resolver = DenoResolver::new(config).await.expect("Failed to create resolver");

    let user = User {
        id: 1,
        name: "John Doe".to_string(),
        email: "john@example.com".to_string(),
        first_name: None,
        last_name: None,
    };

    let result: String = resolver
        .call_field_resolver("user_resolver.js", "displayName", &user, &EmptyArgs {})
        .await
        .expect("Failed to call resolver");

    assert_eq!(result, "John Doe");
}

#[tokio::test]
async fn test_display_name_falls_back_to_email() {
    let config = DenoConfig {
        base_dir: fixtures_dir(),
        ..Default::default()
    };

    let resolver = DenoResolver::new(config).await.expect("Failed to create resolver");

    let user = User {
        id: 2,
        name: "".to_string(), // Empty name
        email: "jane@example.com".to_string(),
        first_name: None,
        last_name: None,
    };

    let result: String = resolver
        .call_field_resolver("user_resolver.js", "displayName", &user, &EmptyArgs {})
        .await
        .expect("Failed to call resolver");

    assert_eq!(result, "jane@example.com");
}

#[tokio::test]
async fn test_full_name_resolver() {
    let config = DenoConfig {
        base_dir: fixtures_dir(),
        ..Default::default()
    };

    let resolver = DenoResolver::new(config).await.expect("Failed to create resolver");

    let user = User {
        id: 3,
        name: "John Doe".to_string(),
        email: "john@example.com".to_string(),
        first_name: Some("John".to_string()),
        last_name: Some("Doe".to_string()),
    };

    let result: String = resolver
        .call_field_resolver("user_resolver.js", "fullName", &user, &EmptyArgs {})
        .await
        .expect("Failed to call resolver");

    assert_eq!(result, "John Doe");
}

#[tokio::test]
async fn test_async_initials_resolver() {
    let config = DenoConfig {
        base_dir: fixtures_dir(),
        ..Default::default()
    };

    let resolver = DenoResolver::new(config).await.expect("Failed to create resolver");

    let user = User {
        id: 4,
        name: "John Doe".to_string(),
        email: "john@example.com".to_string(),
        first_name: None,
        last_name: None,
    };

    let result: String = resolver
        .call_field_resolver("user_resolver.js", "initials", &user, &EmptyArgs {})
        .await
        .expect("Failed to call resolver");

    assert_eq!(result, "JD");
}

#[tokio::test]
async fn test_root_resolver() {
    let config = DenoConfig {
        base_dir: fixtures_dir(),
        ..Default::default()
    };

    let resolver = DenoResolver::new(config).await.expect("Failed to create resolver");

    let args = GreetingArgs {
        name: Some("World".to_string()),
    };

    let result: String = resolver
        .call_root_resolver("user_resolver.js", "greeting", &args)
        .await
        .expect("Failed to call resolver");

    assert_eq!(result, "Hello, World!");
}

#[tokio::test]
async fn test_root_resolver_default_name() {
    let config = DenoConfig {
        base_dir: fixtures_dir(),
        ..Default::default()
    };

    let resolver = DenoResolver::new(config).await.expect("Failed to create resolver");

    let args = GreetingArgs { name: None };

    let result: String = resolver
        .call_root_resolver("user_resolver.js", "greeting", &args)
        .await
        .expect("Failed to call resolver");

    assert_eq!(result, "Hello, World!");
}
