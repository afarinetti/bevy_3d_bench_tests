# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

This is a Rust project for 3D benchmarking tests using the Bevy game engine. Currently in early setup phase.

## Build Commands

```bash
# Build the project
cargo build

# Build with optimizations (recommended for benchmarking)
cargo build --release

# Run the application
cargo run

# Run with release optimizations
cargo run --release

# Run tests
cargo test

# Run a specific test
cargo test test_name

# Check code without building
cargo check

# Format code
cargo fmt

# Run linter
cargo clippy
```

## Bevy-Specific Notes

When adding Bevy as a dependency, use dynamic linking during development for faster compile times:

```toml
# In Cargo.toml for development
[dependencies]
bevy = { version = "0.15", features = ["dynamic_linking"] }
```

Disable dynamic linking for release builds.
