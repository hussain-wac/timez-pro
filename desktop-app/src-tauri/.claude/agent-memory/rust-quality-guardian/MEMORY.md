# Rust Quality Guardian Memory - Timez Pro Desktop App

## Project Architecture

### Service-Based IPC Pattern
The desktop app uses separate service processes communicating via Unix domain sockets (Linux/macOS) or TCP ports (Windows):
- **timez-auth-service** - Authentication state management
- **timez-task-service** - Task and timer management, midnight reset
- **timez-tracker-service** - Activity tracking
- **timez-idle-time-service** - Idle detection
- **timez-quit-service** - Graceful shutdown

### Key Files
- `/core/src/timer_state.rs` - TimerStateInner manages all timer state including `base_elapsed` HashMap
- `/core/src/protocol.rs` - Request/ResponseData enums for IPC
- `/core/src/models.rs` - Shared data models
- `/service/src/servers/task.rs` - Task service implementation with sync thread
- `/app/src/lib.rs` - Tauri app setup and event bridge
- `/app/src/ipc.rs` - IPC service manager and request routing

### Event Flow
1. Tauri app spawns `spawn_event_bridge` thread polling every 2 seconds
2. Events emitted via `app_handle.emit()` to frontend React app
3. Events: `midnight-reset`, `timer-stopped`, `idle-detected`, `activity-update`

## Patterns Learned

### Local Date Tracking for Midnight Reset
Use `chrono::Local::now().date_naive()` for local timezone-aware date comparison.
Store `current_day: NaiveDate` field and compare on each sync cycle.

### Proper Reset Sequence
1. Sync any running timer to backend BEFORE clearing state
2. Stop the timer (clear `running_task_id` and `timer_started_at`)
3. Clear `base_elapsed` and `last_synced_elapsed` HashMaps
4. Update `current_day` to today
5. Emit event to frontend

### Request Routing
New requests must be added to:
1. `core/src/protocol.rs` - Request enum
2. `app/src/ipc.rs` - `route_request()` function
3. Service handler (e.g., `service/src/servers/task.rs`)

### Response Types
New response types must be added to:
1. `core/src/models.rs` - Model struct
2. `core/src/protocol.rs` - ResponseData enum
3. `app/src/ipc.rs` - decoder function (e.g., `decode_midnight_reset()`)

## Common Anti-patterns Avoided

### Unreliable Midnight Check (Fixed)
Old code checked `now.hour() == 0 && now.minute() == 0` which is unreliable with 30-second intervals.
Fixed by comparing `NaiveDate` values which properly detects any day change.

### Missing Backend Sync Before Reset
Always sync accumulated time to backend before clearing local state.
