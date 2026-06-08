#!/bin/bash
# SessionStart hook for Claude Code on the web.
# Prepares the Rust nightly toolchain so `cargo check`, `cargo test --lib`,
# clippy and rustfmt work out of the box for the silent-breath-mmio crate.
set -euo pipefail

# Only run in the remote (web) environment; local machines already have a setup.
if [ "${CLAUDE_CODE_REMOTE:-}" != "true" ]; then
  exit 0
fi

cd "$CLAUDE_PROJECT_DIR"

# rust-toolchain.toml pins nightly; make sure it is installed and is the
# active default, then add the components the project needs:
#   - clippy / rustfmt : linting and formatting (used in validation)
#   - rust-src         : required by baremetal-abi's build-std (.cargo/config.toml)
rustup toolchain install nightly --profile minimal --no-self-update
rustup default nightly
rustup component add clippy rustfmt rust-src

# Pre-download crate dependencies so the first build is offline-fast.
cargo fetch --locked 2>/dev/null || cargo fetch

# Warm the build cache for the root crate (mirrors CI: `cargo check --all-targets`).
# The container filesystem is cached after the hook completes, so this makes the
# first in-session `cargo test` / `cargo clippy` near-instant.
cargo check --all-targets

echo "session-start: Rust nightly toolchain + components ready."
