# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build Commands
- Build: `bash scripts/build.sh [debug|release]` (default: release)
- Flash: `bash scripts/flash.sh [debug|release]` (default: release)
- For individual tests: N/A (no test suite configured)
- For linting: `cargo clippy`
- For code formatting: `cargo fmt`

## Code Style
- Rust 2021 edition with compiler version 1.77+
- ESP32 specific toolchain (channel "esp") required
- Optimize for size in release builds (`opt-level = "s"`)
- Use the log crate for logging (`log::info!`, etc.)
- Always initialize ESP logger via `esp_idf_svc::log::EspLogger::initialize_default()`
- Always call `esp_idf_svc::sys::link_patches()` first in main
- Prefer embedded-specific APIs from the esp-idf-svc crate
- Keep functions small and focused on single responsibilities
- Use standard Rust naming conventions (snake_case for functions/variables, CamelCase for types)