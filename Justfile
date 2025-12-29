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

# =============================================================================
# Docker
# =============================================================================

# Start postgres via docker-compose
db-up:
    docker-compose up -d postgres

# Stop postgres
db-down:
    docker-compose down

# View postgres logs
db-logs:
    docker-compose logs -f postgres

# =============================================================================
# Example (unified example with IAM + Blog services)
# =============================================================================

# Database URL for example
example_db := "postgres://postgres:postgres@localhost/synapse_unified"

# Build the example (all features)
example-build: build-release
    cd examples/unified && cargo build

# Alias for unified-build
unified-build: example-build

# Build the example in release mode
example-build-release: build-release
    cd examples/unified && cargo build --release

# Run the monolith (all services + gateway in one process)
example-run: build-release db-up
    cd examples/unified && DATABASE_URL="{{example_db}}" cargo run --bin monolith

# Alias for example-run
monolith: example-run

# Run just the gateway (connects to external gRPC services)
gateway: build-release
    cd examples/unified && cargo run --bin gateway

# Run just the IAM gRPC service
iam: build-release db-up
    cd examples/unified && DATABASE_URL="{{example_db}}" cargo run --bin iam-service

# Run just the Blog gRPC service
blog: build-release db-up
    cd examples/unified && DATABASE_URL="{{example_db}}" cargo run --bin blog-service

# Run services as microservices (IAM + Blog + Gateway separately)
demo: build-release db-up
    @echo "Starting IAM service on :50051..."
    @cd examples/unified && DATABASE_URL="{{example_db}}" cargo run --bin iam-service &
    @sleep 2
    @echo "Starting Blog service on :50052..."
    @cd examples/unified && DATABASE_URL="{{example_db}}" cargo run --bin blog-service &
    @sleep 2
    @echo "Starting Gateway on :4000..."
    @cd examples/unified && cargo run --bin gateway
