# Rust Quality Guardian Memory - Timez Pro Desktop App

## Project Architecture

The desktop app uses a **multi-process service architecture**:
- Main Tauri app (`app/`) communicates with background services via IPC
- Unix: Unix domain sockets at `/tmp/timez-*.sock`
- Windows: TCP localhost ports 23400-23405

### Crates
- `core/` - Shared types: models, protocol, API client, idle detection
- `service/` - Background service binaries (auth, task, tracker, idle-time, quit)
- `app/` - Main Tauri application

## Critical Cross-Platform Patterns

### File Paths
- **NEVER** hardcode Unix paths like `/tmp/` for Windows compatibility
- Use `std::env::temp_dir()` on Windows
- Use `OnceLock` for lazily-initialized static paths (see `auth_store.rs`)

### Windows Service Spawning
- Use `CREATE_NO_WINDOW` (0x08000000) creation flag via `CommandExt`
- Always pass `--parent-pid` for watchdog functionality

### Windows Idle Detection
- `GetForegroundWindow() == 0` is NOT reliable for lock detection
- Use `OpenInputDesktop` with `DESKTOP_SWITCHDESKTOP` flag instead

### TCP/Socket Binding
- Windows: Add retry logic for port binding (zombi process cleanup)
- Set read timeouts to prevent hanging on malformed requests
- Unix: Always remove stale sockets before binding

## Common Bugs Fixed

### Timer State Bug
**Issue**: `stop_current()` called `stop_current_local()` which cleared `timer_started_at`, then tried to read it afterward.
**Fix**: Return a `StopInfo` struct from `stop_current_local()` containing all needed data.

### Auth Store Windows Path
**Issue**: Hardcoded `/tmp/timez-auth-store.json` fails on Windows.
**Fix**: Use platform-conditional path with `std::env::temp_dir()` for Windows.

## Clippy Guidelines

Enable these for production:
- `clippy::all` (standard)
- `clippy::pedantic` for library code (with allowances for docs in internal code)

Key pedantic lints to watch:
- `ref_option` - prefer `Option<&T>` over `&Option<T>`
- `needless_pass_by_value` - take references where possible
- `cast_possible_truncation` - explicit truncation handling
- `missing_errors_doc` - document error conditions

## Testing Checklist for Windows

1. Service binary spawning (check CREATE_NO_WINDOW flag)
2. TCP port binding (ports 23400-23405)
3. Auth token storage (%TEMP% directory)
4. Workstation lock detection
5. Single-instance handling

## Error Handling Patterns

- Services should log startup errors to stderr
- Main app writes crash logs to `%TEMP%/timez-pro-crash.log` on Windows
- All IPC connections should have read timeouts
