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

## Code Quality Observations
- Uses proper Mutex for thread safety
- Good separation between local state and API sync
- Timestamp array capped at 100 entries to prevent unbounded growth
