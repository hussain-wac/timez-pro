# Timez Pro - Architecture Documentation

## Overview

Timez Pro is a desktop time tracking application built with **Tauri 2** (Rust backend + React/TypeScript frontend). It integrates with a backend API to manage tasks, track time, detect idle periods across all platforms, and provide activity statistics with robust offline-first sync capabilities.

---

## High-Level Architecture

```
┌─────────────────────────────────────────────────────────────────────┐
│                        Desktop Application                          │
├─────────────────────────────────────────────────────────────────────┤
│  ┌─────────────────────────────┐    ┌────────────────────────────┐  │
│  │    React Frontend (TSX)    │◄──►│    Tauri Commands (Rust)  │  │
│  │                             │    │                            │  │
│  │  - App.tsx (Main UI)       │    │  - commands.rs            │  │
│  │  - LoginPage.tsx           │    │  - services/tasks.rs      │  │
│  │  - AuthContext.tsx         │    │  - services/auth.rs       │  │
│  │  - App.css                 │    │  - services/activity.rs   │  │
│  └─────────────────────────────┘    └────────────────────────────┘  │
│                   │                              │                    │
│                   ▼                              ▼                    │
│  ┌───────────────────────────────────────────────────────────────┐   │
│  │                    Rust Backend Core                          │   │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐   │   │
│  │  │ timer_state │  │local_store  │  │         api         │   │   │
│  │  │   .rs       │  │   .rs       │  │         .rs         │   │   │
│  │  └─────────────┘  └─────────────┘  └─────────────────────┘   │   │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐   │   │
│  │  │  models.rs  │  │ services/   │  │     lib.rs          │   │   │
│  │  │             │  │ quit.rs     │  │   (Tauri Setup)     │   │   │
│  │  └─────────────┘  └─────────────┘  └─────────────────────┘   │   │
│  └───────────────────────────────────────────────────────────────┘   │
│                              │                                        │
│  ┌───────────────────────────────────────────────────────────────┐   │
│  │                    Service Processes                          │   │
│  │  ┌─────────────┐  ┌──────────────┐  ┌──────────────────────┐  │   │
│  │  │ auth-service│  │task-service  │  │ idle-time-service   │  │   │
│  │  └─────────────┘  └──────────────┘  └──────────────────────┘  │   │
│  │  ┌─────────────┐  ┌──────────────┐                            │   │
│  │  │quit-service │  │tracker-service│                           │   │
│  │  └─────────────┘  └──────────────┘                            │   │
│  └───────────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────────┘
                              │
                              ▼
                    ┌─────────────────┐
                    │   Backend API   │
                    │  (FastAPI)      │
                    │ 192.168.3.163   │
                    │    :8000        │
                    └─────────────────┘
```

---

## Technology Stack

### Frontend
| Technology | Purpose |
|------------|---------|
| React 19 | UI framework |
| TypeScript | Type safety |
| Tailwind CSS 4 | Styling |
| Vite 7 | Build tool |
| @tauri-apps/api | Tauri IPC communication |

### Backend (Desktop)
| Technology | Purpose |
|------------|---------|
| Tauri 2 | Desktop runtime |
| Rust | Backend logic |
| serde | Serialization |
| chrono | Date/time handling |
| dbus (Linux) | System idle detection |
| ioreg (macOS) | System idle detection |
| Win32 FFI (Windows) | System idle detection |
| ureq | HTTP client |
| tauri-plugin-notification | Desktop notifications |

---

## Module Analysis

### Frontend Modules

#### `src/main.tsx`
Entry point that sets up React with AuthProvider and conditional rendering between LoginPage and App.

#### `src/AuthContext.tsx`
Manages authentication state:
- Stores user info, access token
- Handles Google OAuth login flow
- Persists token in localStorage
- Provides `loginViaBrowser()` and `logout()` methods

#### `src/LoginPage.tsx`
Simple login UI with Google OAuth button. Initiates OAuth flow via AuthContext.

#### `src/App.tsx`
Main application UI with:
- **Left sidebar**: User profile, total runtime, task breakdown
- **Right panel**: Task list with search, timer controls
- **Modals**: Idle time alert, quit confirmation
- **Event listeners**: Listens to Tauri events (`idle-detected`, `timer-stopped`, `activity-update`, `sync-complete`, `midnight-reset`)

---

### Rust Backend Modules

#### `src-tauri/src/main.rs`
- Single-instance enforcement (prevents multiple app instances)
- Calls `timez_pro_lib::run()`

#### `src-tauri/src/lib.rs`
Main Tauri application setup:
- **State management**: `AuthToken`, `ActivityState`, `TimerState`, `LocalTimeStorage`
- **System tray**: Menu with Show/Pause-Resume/Quit options
- **Background threads**:
  - Sync thread (30-second intervals)
  - Timestamp thread (5-second intervals)
  - Idle monitor thread (60-second threshold)
- **Window events**: Hide instead of close (stays in tray)
- **Command handlers**: Tauri commands exposed to frontend

#### `src-tauri/src/commands.rs`
Thin wrapper around service functions. Each command delegates to corresponding service module.

#### `src-tauri/src/models.rs`
Data structures for frontend-backend communication:
```rust
Task, TimerStatus, IdleEvent, ActivityStats, AuthUser, AuthResponse
```

#### `src-tauri/src/api.rs`
HTTP client for backend API (`http://192.168.3.163:8000`):
- `google_login()` - Exchange Google ID token for access token
- `get_me()` - Validate token, get user info
- `list_tasks()` - Fetch tasks with elapsed times (in_progress status only)
- `sync_time()` - Sync cumulative time with handshake confirmation
- `get_status()` - Get current running task
- `google_oauth_via_browser()` - OAuth2 flow with local callback server

**Handshake Confirmation**: The `sync_time()` function returns a `SyncTimeResponse` that confirms the backend received the data:
```rust
pub struct SyncTimeResponse {
    pub id: i64,
    pub task_id: i64,
    pub duration: Option<i64>,
    pub is_synced: bool,
}
```

#### `src-tauri/src/timer_state.rs`
Local timer state management with robust error handling:

**Mutex Poisoning Recovery**:
```rust
trait RecoverableMutex<T> {
    fn lock_or_recover(&self) -> Result<std::sync::MutexGuard<'_, T>, String>;
}
```
Uses `clear_poison()` to recover from panics in other threads.

**Core State**:
- `cached_tasks` - Local copy of tasks from API
- `running_task_id` - Currently active timer
- `timer_started_at` - Local start timestamp
- `base_elapsed` - Time accumulated before current run
- `last_sync_at` - Track sync timing

**Background Threads**:
- **Sync Thread**: Every 30 seconds, syncs unsynced entries to backend with handshake confirmation
- **Timestamp Thread**: Every 5 seconds, records elapsed time for recovery
- **Midnight Reset**: Automatically stops timers at midnight

**Non-Blocking Design**: Releases mutex before network I/O to prevent UI freezes.

**Desktop Notifications**: Sends notifications via `tauri-plugin-notification` for:
- Sync in progress
- Sync success with duration
- Sync failures

#### `src-tauri/src/local_store.rs`
Persistent local storage for time entries:
- Stores timestamps with elapsed seconds
- Tracks sync state (`last_synced_elapsed`)
- Handles `client_started_at` and `client_stopped_at` for each entry
- Cleanup completed entries after sync confirmation

#### `src-tauri/src/services/`
| Module | Responsibility |
|--------|----------------|
| `tasks.rs` | Task CRUD, timer control |
| `auth.rs` | OAuth flow, token validation, logout |
| `activity.rs` | Activity statistics retrieval |
| `quit.rs` | Non-blocking quit with sync-before-exit |

#### `src-tauri/src/services/quit.rs`
Handles application quit with time sync:
- Captures running task data before releasing mutex
- Spawns background thread for sync (non-blocking)
- Retry logic: up to 3 attempts with 300ms delays
- Returns immediately while sync completes in background
- Desktop notification on sync completion

---

## Cross-Platform Idle Detection

The idle detection service (`timez-idle-time-service`) supports all major platforms:

### Linux
- Uses **D-Bus** to query GNOME Mutter IdleMonitor
- Path: `/org/gnome/Mutter/IdleMonitor/Core`
- Method: `GetIdletime()` returns milliseconds

### macOS
- Uses **ioreg** command to query `HIDIdleTime`
- Command: `ioreg -c IOHIDSystem | grep HIDIdleTime`
- Returns nanoseconds, converted to seconds

### Windows
- Uses **Win32 FFI** with `GetLastInputInfo`
- Calculates idle from `GetTickCount() - dwTime`
- No external processes required

### Idle Flow
```
1. Idle monitor polls every 2 seconds
2. If idle > 60 seconds:
   - Stop current timer locally
   - Emit "idle-detected" event with idle duration
3. Frontend shows modal to keep/discard idle time
4. On user action:
   - Keep: Call "add_idle_time" (adds idle to task time, resumes timer)
   - Discard: Call "discard_idle_time" (resumes without adding time)
```

---

## Data Flow

### Authentication Flow
```
1. User clicks "Sign in with Google" (LoginPage.tsx)
2. AuthContext.loginViaBrowser() → Tauri command "start_google_auth"
3. Rust opens system browser → Google OAuth consent page
4. Google redirects to local callback server (api.rs)
5. Backend exchanges code for ID token → API for access token
6. Token stored in AuthToken state + localStorage
7. TimerState syncs tasks from API
8. App renders main UI with tasks
```

### Timer Flow
```
1. User clicks play button (App.tsx)
2. Tauri command "start_timer" → services/tasks.rs
3. Local timer state updates immediately (no API call)
4. LocalTimeStorage records start time
5. Frontend receives updated task list (instant UI)
6. Timestamp thread records elapsed every 5 seconds
7. Sync thread pushes to API every 30 seconds
```

### Sync Flow
```
1. Sync thread wakes every 30 seconds
2. Get unsynced entries from LocalTimeStorage
3. For each entry:
   a. Send notification "Syncing X to server..."
   b. Call sync_time() API with cumulative elapsed
   c. Verify handshake (task_id matches)
   d. Mark as synced in LocalTimeStorage
   e. Send notification "X synced successfully"
4. Refresh task cache from API
```

### Quit Flow
```
1. User triggers quit (tray menu or window close)
2. Capture running task data (short lock)
3. Release mutex immediately
4. Spawn background thread:
   a. Attempt sync_time() up to 3 times
   b. 300ms delay between retries
   c. Send desktop notification on result
5. Return immediately (non-blocking UI)
6. App exits cleanly
```

---

## State Management

### Frontend State
- **React Context**: `AuthContext` for authentication
- **useState**: Local UI state (tasks, selectedTaskId, modals)
- **useEffect**: Event listeners, polling intervals

### Backend State (Tauri State)
| State | Type | Purpose |
|-------|------|---------|
| `AuthToken` | `Mutex<AuthTokenState>` | Access token storage |
| `ActivityState` | `Mutex<ActivityTracker>` | Active/idle time tracking |
| `TimerState` | `Mutex<TimerStateInner>` | Task cache, running timer |
| `LocalTimeStorage` | (atomic operations) | Persistent time entries |

### Mutex Safety
All mutex operations use the `lock_or_recover()` pattern to handle poisoning:
```rust
fn lock_or_recover(&self) -> Result<MutexGuard<'_, T>, String> {
    self.lock().map_err(|e| {
        eprintln!("[CRITICAL] Mutex poisoned, recovering: {}", e);
        format!("Mutex poisoned: {}", e)
    }).or_else(|_| {
        self.clear_poison();
        self.lock().map_err(|e| format!("Failed to recover mutex: {}", e))
    })
}
```

---

## Key Design Decisions

1. **Offline-First Architecture**: Timer runs locally with periodic background sync; no network required for basic operation
2. **Non-Blocking Sync**: Network I/O happens outside mutex locks; UI never freezes
3. **Cross-Platform Idle Detection**: Native APIs for each platform (D-Bus, ioreg, Win32)
4. **Handshake Confirmation**: Every sync verifies backend received correct data
5. **Graceful Shutdown**: Sync-before-quit with retry logic ensures no time loss
6. **Mutex Poisoning Recovery**: `clear_poison()` prevents cascading failures
7. **Single Instance**: Prevents multiple app instances via `single-instance` crate
8. **System Tray**: App hides to tray on close instead of exiting
9. **OAuth via Browser**: Full OAuth2 flow with local callback server
10. **30-Second Sync Interval**: Balances freshness with API load
11. **5-Second Timestamps**: Fine-grained recovery for unexpected crashes
12. **Desktop Notifications**: User feedback for sync status

---

## API Endpoints (Backend)

| Endpoint | Method | Purpose |
|----------|--------|---------|
| `/api/auth/google` | POST | Exchange Google token |
| `/api/auth/me` | GET | Validate token, get user |
| `/api/tasks` | GET | List all tasks |
| `/api/tasks/timer` | GET | List in_progress tasks for timer |
| `/api/tasks/sync-time` | POST | Upsert time entry with handshake |
| `/api/tasks/{id}` | GET | Get single task |
| `/api/tasks/{id}` | DELETE | Delete task |
| `/api/tasks/{id}/entries` | GET | List time entries for task |
| `/api/status` | GET | Current running status |
| `/api/report/summary` | GET | Time summary |

### Sync Time Request/Response
```json
// Request
{
  "task_id": 123,
  "elapsed_seconds": 3600,
  "client_started_at": "2024-01-15T10:00:00Z",
  "client_stopped_at": "2024-01-15T11:00:00Z"  // null if still running
}

// Response (handshake confirmation)
{
  "id": 456,
  "task_id": 123,
  "duration": 3600,
  "is_synced": true
}
```

---

## Build & Distribution

- **Frontend**: Vite builds to `dist/`
- **Backend**: Cargo builds Rust to binary
- **Bundle**: Tauri bundles as `.deb` (Linux), `.dmg` (macOS), `.exe` (Windows)
- **Services**: Additional service binaries bundled as resources:
  - `timez-auth-service`
  - `timez-task-service`
  - `timez-tracker-service`
  - `timez-idle-time-service`
  - `timez-quit-service`
- **Icons**: Square PNG icons (256x256, 128x128, 32x32) + .icns (macOS) + .ico (Windows)

---

## Security Considerations

- Tokens stored in localStorage (frontend) and Tauri state (backend)
- API requests include Bearer token
- CSP disabled in tauri.conf.json (for development)
- Single-instance prevents token conflicts
- OAuth2 with system browser (no embedded webview)
- Local callback server only listens on 127.0.0.1

---

## Error Handling

### Network Failures
- Local timer continues running
- Sync retries in background
- Desktop notification on failure
- Data persists in LocalTimeStorage until sync succeeds

### Mutex Poisoning
- Automatic recovery via `clear_poison()`
- Logged as `[CRITICAL]` for debugging
- State remains accessible after recovery

### Quit During Sync
- Background thread continues sync
- Up to 3 retry attempts
- App exits after best-effort sync

---

## Future Improvements

- Add encryption for stored tokens
- Implement task creation/deletion from app
- Add reporting and analytics UI
- Support for multiple workspaces
- Configurable sync intervals
- Push notifications from server
