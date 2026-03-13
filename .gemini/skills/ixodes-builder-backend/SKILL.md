---
name: ixodes-builder-backend
description: Expert in modifying the Tauri-based backend for the Ixodes GUI builder.
---

# Ixodes Builder Backend

You are an expert in the `ixodes-gui` project's Rust/Tauri backend. Your goal is to help users extend the builder's capabilities, modify the compilation process, or integrate new build-time configurations.

## Core Concepts

- **Tauri Commands:** Defined in `ixodes-gui/src-tauri/src/lib.rs` and registered in `run()`.
- **BuildRequest:** The data structure passed from the frontend containing all build settings and branding options.
- **Resource Injection:** The process of appending the encrypted configuration and optional "pump" data (zero-padding) to the end of the built executable.
- **Branding:** Handled via environment variables passed to `cargo build` and processed by the `ixodes` agent's `build.rs`.

## Key Files

- `ixodes-gui/src-tauri/src/lib.rs`: Contains all logic for building, branding, and resource injection.
- `ixodes/build.rs`: (In the agent) Consumes the branding environment variables.

## Workflow for Adding a New Build Setting

1.  **Update Data Structures:**
    Modify `RecoverySettings` and `PayloadConfig` in `ixodes-gui/src-tauri/src/lib.rs` to include the new field.

2.  **Update Build Logic:**
    In `build_ixodes_sync`, ensure the new field is correctly mapped from `BuildRequest` to `PayloadConfig`.

3.  **Update Agent-side (if needed):**
    If the setting requires a compile-time feature, add it to the `cargo build` arguments in `build_ixodes_sync`.

4.  **Resource Injection:**
    The `PayloadConfig` is serialized to JSON, encrypted (XOR), and appended to the binary. Ensure any new fields are serializable.

## Best Practices

- **Blocking vs. Async:** Long-running operations (like `cargo build`) must be run in `spawn_blocking`.
- **Error Propagation:** Return `Result<T, String>` from Tauri commands to properly show errors in the UI.
- **Path Management:** Use `tauri::path::BaseDirectory` and `app.path()` to resolve resource paths reliably.
- **Cross-Platform:** Always check `cfg!(windows)` if adding OS-specific functionality.
