# Synapse Justfile

# Default recipe
default: build

# Build protoc-gen-synapse in debug mode
build:
    cd protoc-gen-synapse && cargo build

# Build protoc-gen-synapse in release mode
build-release:
    cd protoc-gen-synapse && cargo build --release

# Run tests
test:
    cd protoc-gen-synapse && cargo test

# Check for errors without building
check:
    cd protoc-gen-synapse && cargo check

# Run clippy lints
lint:
    cd protoc-gen-synapse && cargo clippy -- -D warnings

# Format code
fmt:
    cd protoc-gen-synapse && cargo fmt

# Format check (for CI)
fmt-check:
    cd protoc-gen-synapse && cargo fmt -- --check

# Generate test output from fixtures
generate: build-release
    rm -rf protoc-gen-synapse/tests/output/*
    protoc \
        --plugin=protoc-gen-synapse=./protoc-gen-synapse/target/release/protoc-gen-synapse \
        --synapse_out=backend=seaorm:protoc-gen-synapse/tests/output \
        -Iproto \
        -Iprotoc-gen-synapse/tests/fixtures \
        protoc-gen-synapse/tests/fixtures/user_service.proto \
        protoc-gen-synapse/tests/fixtures/user.proto \
        protoc-gen-synapse/tests/fixtures/graphql_user.proto \
        protoc-gen-synapse/tests/fixtures/graphql_uuid.proto

# Generate with debug output
generate-debug: build-release
    rm -rf protoc-gen-synapse/tests/output/*
    SEAORM_DEBUG=1 protoc \
        --plugin=protoc-gen-synapse=./protoc-gen-synapse/target/release/protoc-gen-synapse \
        --synapse_out=backend=seaorm:protoc-gen-synapse/tests/output \
        -Iproto \
        -Iprotoc-gen-synapse/tests/fixtures \
        protoc-gen-synapse/tests/fixtures/user_service.proto

# Clean build artifacts
clean:
    cd protoc-gen-synapse && cargo clean
    rm -rf protoc-gen-synapse/tests/output/*

# Full CI check
ci: fmt-check lint test

# Watch for changes and rebuild
watch:
    cd protoc-gen-synapse && cargo watch -x build

# Show generated files
show-output:
    find protoc-gen-synapse/tests/output -name "*.rs" -exec echo "=== {} ===" \; -exec cat {} \;

# Build the full-stack example
example-build: build-release
    cd examples/full-stack && cargo build

# Run the full-stack example server
example-run: build-release
    cd examples/full-stack && cargo run

# Run the full-stack example with watch (rebuilds on changes)
example-watch: build-release
    cd examples/full-stack && cargo watch -x run
