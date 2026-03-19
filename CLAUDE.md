# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

macOS command-line tool that gracefully quits all running GUI applications. Written in Rust, using the `objc2` crate family to interact with macOS AppKit/NSWorkspace APIs directly.

## Build & Run

```bash
cargo build --release        # Build optimized binary at target/release/quit-all
cargo run --release           # Build and run
cargo run --release -- --dry-run   # Preview which apps would be quit
cargo run --release -- --force     # Force-terminate apps (no save prompt)
```

No tests currently exist. No linter or formatter configuration — use `cargo fmt` and `cargo clippy`.

## Architecture

Single-file app (`src/main.rs`). Uses `NSWorkspace.sharedWorkspace().runningApplications()` to enumerate running apps, filters to only regular-activation-policy apps (visible GUI apps), skips safelisted bundle IDs and its own PID, then calls `terminate()` or `forceTerminate()` on each.

Key constants: `SAFELIST` — bundle IDs that are never quit (Finder, System Preferences).

## Platform

macOS only. Depends on Objective-C runtime bindings (`objc2`, `objc2-foundation`, `objc2-app-kit`) and `libc` for `getpid()`.
