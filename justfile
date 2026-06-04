# vhdx-compactor — task runner. Run `just` (or `just --list`) to see recipes.
# All recipes assume you are inside `nix develop`.

set shell := ["bash", "-cu"]

# Show available recipes (default when run with no args).
default:
    @just --list

# Build the whole workspace (debug) for the host.
build:
    cargo build --workspace

# Cross-compile the GUI binary to Windows (release).
build-win:
    cargo build -p vhdx-compactor --target x86_64-pc-windows-gnu --release

# Run the host unit-test suite (exercises vhdx-core + pipeline policy).
test:
    cargo test --workspace

# Clippy on the host target.
lint:
    cargo clippy --workspace --all-targets -- -D warnings

# Clippy on the Windows cross target (catches cfg(windows) issues).
lint-win:
    cargo clippy -p vhdx-compactor --all-targets --target x86_64-pc-windows-gnu -- -D warnings

# Apply rustfmt.
fmt:
    cargo fmt --all

# Verify rustfmt would not change anything (used in CI).
fmt-check:
    cargo fmt --all -- --check

# Run the GUI on the host (shows the empty-state UI; WSL actions are Windows-only).
run:
    cargo run -p vhdx-compactor

# What CI runs — fast feedback before pushing.
ci: fmt-check lint lint-win test
    @echo "✓ all checks passed"
