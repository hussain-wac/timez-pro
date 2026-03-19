# Timez Pro - Rust Quality Guardian Memory

## Project Overview
- Tauri desktop app for time tracking
- Location: `/home/wac/Desktop/timez-pro/desktop-app/src-tauri/`
- Uses local storage with periodic API syncing

## Critical Bug Patterns Found

### Timestamp Cumulative vs Delta Bug (Fixed 2026-03-18)
**Pattern**: Code incorrectly summed cumulative timestamps instead of taking the last value.

**Root cause**: The timestamp recording thread stores `elapsed_secs` as the TOTAL time since timer started (cumulative), not incremental deltas. Sync logic was using `.iter().sum()` instead of `.last()`.

**Example of wrong code**:
```rust
let total_elapsed: i64 = entry.timestamps.iter().map(|t| t.elapsed_secs).sum();
// If timestamps are [5, 10, 15], this returns 30 instead of 15
```

**Correct code**:
```rust
let total_elapsed: i64 = entry.timestamps.last().map(|t| t.elapsed_secs).unwrap_or(0);
```

**Files affected**:
- `src/timer_state.rs` (sync thread, line ~227)
- `src/services/quit.rs` (quit handler, line ~38)
- `src/services/timer.rs` (sync logic, line ~205)

## Architecture Notes

### Timer Data Flow
1. `spawn_timestamp_thread` records snapshots every 5 seconds with cumulative elapsed time
2. `spawn_sync_thread` syncs to API every 30 seconds (was 60, reduced for better reliability)
3. `quit_app` does final sync before exit
4. `crash_recovery_on_startup` handles incomplete sessions

### Key Data Structures
- `TimeStamp { timestamp: String, elapsed_secs: i64 }` - cumulative, not delta
- `LocalTimeStorage` - persists unsynced entries to disk
- `TimerStateInner` - in-memory state with base_elapsed map

### Service Architecture (IPC)
- Uses Unix domain sockets on Linux/macOS, TCP ports on Windows
- Services: Auth (23401), Task (23402), Tracker (23403), IdleTime (23404), Quit (23405)
- Protocol: JSON-serialized `RequestEnvelope` / `ResponseEnvelope`

### Adding New Features Checklist
1. **core/src/models.rs** - Add data structs with `#[derive(Debug, Clone, Serialize, Deserialize)]`
2. **core/src/protocol.rs** - Add Request/ResponseData enum variants
3. **core/src/api.rs** - Add API client functions
4. **service/src/servers/*.rs** - Add handler functions in relevant service
5. **app/src/lib.rs** - Add Tauri commands with `#[tauri::command]`
6. **app/src/ipc.rs** - Add routing in `route_request()` and decoder function
7. **React frontend** - Add TypeScript interfaces and invoke calls

## Project-Based Architecture (Added 2026-03-19)

### New Models
- `Project { id, name, color, task_count }`
- `Task` now has `project_id: Option<i64>` and `project_name: Option<String>`

### New API Endpoints Expected
- `GET /api/me/projects` - User's allocated projects
- `GET /api/projects/{id}/tasks` - Tasks in a project

### New Protocol Variants
- `Request::ListProjects`
- `Request::ListProjectTasks { project_id }`
- `Request::SetActiveProject { project_id }`
- `ResponseData::Projects(Vec<Project>)`

## Code Quality Observations
- Uses proper Mutex for thread safety
- Good separation between local state and API sync
- Timestamp array capped at 100 entries to prevent unbounded growth
- Task fields use `Option<T>` for backward compatibility with API

## Testing Commands
```bash
cd /home/wac/Desktop/timez-pro/desktop-app/src-tauri
cargo check              # Check all crates
cargo check -p timez-service  # Check service crate
npx tsc --noEmit         # TypeScript type check (from desktop-app/)
```
