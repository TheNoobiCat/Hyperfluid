# Hyperfluid build targets
# Run `just` (no args) to see available targets.

default:
    @just --list

# === Build ===

build:
    cargo build --workspace

build-release:
    cargo build --workspace --release

# === Test ===

test:
    cargo test --workspace

test-release:
    cargo test --workspace --release

test-doc:
    cargo test --workspace --doc

# === Formatting ===

fmt:
    cargo fmt --all -- --check

fmt-fix:
    cargo fmt --all

# === Linting ===

lint:
    cargo clippy --workspace --all-targets -- -D warnings

lint-fix:
    cargo clippy --workspace --all-targets --fix --allow-dirty --allow-staged

# === Documentation ===

doc:
    cargo doc --workspace --no-deps --document-private-items

doc-open:
    cargo doc --workspace --no-deps --document-private-items --open

# === Benchmarking ===

bench:
    cargo bench --workspace

# === Audit ===

audit:
    cargo deny check

audit-update:
    cargo deny check advisories

# === Cleanup ===

clean:
    cargo clean

# === CI targets (combined) ===

ci-build:
    cargo build --workspace

ci-test:
    cargo test --workspace

ci-fmt:
    cargo fmt --all -- --check

ci-lint:
    cargo clippy --workspace --all-targets -- -D warnings

ci-audit:
    cargo deny check

ci-doc:
    cargo doc --workspace --no-deps --document-private-items

ci: ci-build ci-test ci-fmt ci-lint ci-audit ci-doc

# === Dev helpers ===

check-all: build test fmt lint audit
