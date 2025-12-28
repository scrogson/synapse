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

# Build the IAM example
iam-build: build-release
    cd examples/iam && cargo build

# Run the IAM example server
iam-run: build-release
    cd examples/iam && cargo run

# Build the gateway example
gateway-build: build-release
    cd examples/gateway && cargo build

# Run the gateway example (requires blog and iam services to be running)
gateway-run: build-release
    cd examples/gateway && cargo run

# Run all services for the gateway demo
# Starts blog service, IAM service, and gateway in the foreground
gateway-demo: build-release
    @echo "Starting Blog service on :50060..."
    @cd examples/full-stack && cargo run &
    @sleep 2
    @echo "Starting IAM service on :50061..."
    @cd examples/iam && cargo run &
    @sleep 2
    @echo "Starting Gateway on :4000..."
    @cd examples/gateway && cargo run

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
# Unified Example (single proto schema, multiple deployment options)
# =============================================================================

# Database URL for unified example
unified_db := "postgres://postgres:postgres@localhost/synapse_unified"

# Build the unified example (all features)
unified-build: build-release
    cd examples/unified && cargo build

# Build the unified example in release mode
unified-build-release: build-release
    cd examples/unified && cargo build --release

# Run the unified monolith (all services + gateway in one process)
unified-monolith: build-release db-up
    cd examples/unified && DATABASE_URL="{{unified_db}}" cargo run --bin monolith

# Run just the unified gateway (connects to external gRPC services)
unified-gateway: build-release
    cd examples/unified && cargo run --bin gateway

# Run just the IAM gRPC service
unified-iam: build-release db-up
    cd examples/unified && DATABASE_URL="{{unified_db}}" cargo run --bin iam_service

# Run just the Blog gRPC service
unified-blog: build-release db-up
    cd examples/unified && DATABASE_URL="{{unified_db}}" cargo run --bin blog_service

# Run unified services as microservices (IAM + Blog + Gateway separately)
unified-demo: build-release db-up
    @echo "Starting IAM service on :50051..."
    @cd examples/unified && DATABASE_URL="{{unified_db}}" cargo run --bin iam_service &
    @sleep 2
    @echo "Starting Blog service on :50052..."
    @cd examples/unified && DATABASE_URL="{{unified_db}}" cargo run --bin blog_service &
    @sleep 2
    @echo "Starting Gateway on :4000..."
    @cd examples/unified && cargo run --bin gateway
