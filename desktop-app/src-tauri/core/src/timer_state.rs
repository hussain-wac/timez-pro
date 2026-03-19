use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use chrono::{Local, NaiveDate, Utc};

use crate::api;
use crate::api::AuthTokenState;
use crate::constants::SYNC_INTERVAL_SECS;
use crate::models::Task;

/// Information returned when stopping a timer locally.
#[derive(Debug)]
pub struct StopInfo {
    pub task_id: i64,
    pub started_at: chrono::DateTime<Utc>,
    pub elapsed_secs: i64,
}

/// Information returned when a midnight reset occurs.
#[derive(Debug)]
pub struct MidnightResetInfo {
    /// Task ID that was synced before reset (if any)
    pub synced_task_id: Option<i64>,
    /// Elapsed seconds that were synced
    pub synced_elapsed: i64,
}

/// Local timer state that tracks everything without hitting the external API.
///
/// The external API is only called every `SYNC_INTERVAL_SECS` for persistence.
/// This design allows the app to function offline and reduces API load.
#[derive(Debug)]
pub struct TimerStateInner {
    /// Cached task list from the last API sync
    pub cached_tasks: Vec<Task>,
    /// Currently running task id
    pub running_task_id: Option<i64>,
    /// When the current timer SESSION was started (for backend session tracking)
    pub session_started_at: Option<chrono::DateTime<Utc>>,
    /// When the last sync slot ended (for calculating incremental slot_seconds)
    pub last_slot_sync_at: Option<chrono::DateTime<Utc>>,
    /// When the current timer was started (local clock for UI display)
    pub timer_started_at: Option<chrono::DateTime<Utc>>,
    /// Last task id we ran (used for resume)
    pub last_task_id: Option<i64>,
    /// Last time we synced with the external API
    pub last_sync_at: chrono::DateTime<Utc>,
    /// Elapsed seconds accumulated before current run (from API summary)
    pub base_elapsed: HashMap<i64, i64>,
    /// Total elapsed sent to backend in last sync (to avoid double-counting)
    pub last_synced_elapsed: HashMap<i64, i64>,
    /// The local date for which we are tracking time (resets at midnight)
    current_day: NaiveDate,
}

pub type TimerState = Mutex<TimerStateInner>;

impl Default for TimerStateInner {
    fn default() -> Self {
        Self::new()
    }
}

impl TimerStateInner {
    /// Creates a new empty timer state.
    #[must_use]
    pub fn new() -> Self {
        Self {
            cached_tasks: Vec::new(),
            running_task_id: None,
            session_started_at: None,
            last_slot_sync_at: None,
            timer_started_at: None,
            last_task_id: None,
            last_sync_at: chrono::DateTime::<Utc>::MIN_UTC,
            base_elapsed: HashMap::new(),
            last_synced_elapsed: HashMap::new(),
            current_day: Local::now().date_naive(),
        }
    }

    /// Returns `true` if a timer is currently running.
    #[must_use]
    #[inline]
    pub fn is_running(&self) -> bool {
        self.running_task_id.is_some()
    }

    /// Maximum reasonable elapsed time for a single session (24 hours in seconds).
    /// Used to detect clock anomalies.
    const MAX_SESSION_SECS: i64 = 24 * 60 * 60;

    /// Maximum total elapsed time we'll track (1 year in seconds).
    /// Prevents overflow issues with very long-running timers.
    const MAX_TOTAL_ELAPSED_SECS: i64 = 365 * 24 * 60 * 60;

    /// Get total elapsed for a running task (base + live)
    ///
    /// This method includes protection against:
    /// - Clock drift (negative values)
    /// - Clock jumps (unreasonably large values)
    /// - Integer overflow (saturating arithmetic)
    pub fn get_total_elapsed(&self, task_id: i64) -> i64 {
        let base = self.base_elapsed.get(&task_id).copied().unwrap_or(0);
        let live = if self.running_task_id == Some(task_id) {
            self.timer_started_at
                .map(|started| {
                    let elapsed = (Utc::now() - started).num_seconds();
                    // Protect against negative values (clock went backward)
                    if elapsed < 0 {
                        return 0;
                    }
                    // Protect against clock jump forward (NTP sync, suspend/resume)
                    // If live elapsed exceeds 24 hours, something is wrong
                    if elapsed > Self::MAX_SESSION_SECS {
                        eprintln!(
                            "[timer] Warning: Detected clock anomaly - live elapsed {} exceeds max session time",
                            elapsed
                        );
                        // Return the max session time as a safe fallback
                        return Self::MAX_SESSION_SECS;
                    }
                    elapsed
                })
                .unwrap_or(0)
        } else {
            0
        };
        // Use saturating_add to prevent overflow
        let total = base.saturating_add(live);
        // Cap at maximum to prevent unreasonable values
        total.min(Self::MAX_TOTAL_ELAPSED_SECS)
    }

    /// Called after successful sync - updates base and resets timer
    pub fn mark_synced(&mut self, task_id: i64, synced_elapsed: i64) {
        // Update base_elapsed to include the synced time
        self.base_elapsed.insert(task_id, synced_elapsed);
        self.last_synced_elapsed.insert(task_id, synced_elapsed);

        // Reset timer_started_at to NOW so live_elapsed starts from 0
        if self.running_task_id == Some(task_id) {
            self.timer_started_at = Some(Utc::now());
        }

        println!(
            "[timer] Marked synced: task_id={}, synced_elapsed={}, timer reset",
            task_id, synced_elapsed
        );
    }

    /// Get tasks with locally computed elapsed time
    ///
    /// Uses the same clock protection as `get_total_elapsed` to ensure consistency.
    pub fn get_tasks(&self) -> Vec<Task> {
        let now = Utc::now();
        let mut out = Vec::with_capacity(self.cached_tasks.len());
        let running_id = self.running_task_id;
        let live_elapsed = if let (Some(_), Some(started)) = (running_id, self.timer_started_at) {
            let elapsed = (now - started).num_seconds();
            // Apply same protections as get_total_elapsed
            if elapsed < 0 {
                0
            } else if elapsed > Self::MAX_SESSION_SECS {
                Self::MAX_SESSION_SECS
            } else {
                elapsed
            }
        } else {
            0
        };

        for t in &self.cached_tasks {
            let base = self
                .base_elapsed
                .get(&t.id)
                .copied()
                .unwrap_or(t.elapsed_secs);
            let is_running = running_id == Some(t.id);
            // Use saturating_add and cap at max
            let total = base.saturating_add(if is_running { live_elapsed } else { 0 });
            out.push(Task {
                id: t.id,
                name: t.name.clone(),
                budget_secs: t.budget_secs,
                elapsed_secs: total.min(Self::MAX_TOTAL_ELAPSED_SECS),
                running: is_running,
                project_id: t.project_id,
                project_name: t.project_name.clone(),
            });
        }

        out
    }

    /// Sync cached data from the external API
    pub fn sync_from_api(&mut self, token: &Option<String>) {
        // Don't sync if no token - this would clear all tasks
        if token.is_none() {
            return;
        }

        if let Ok(tasks) = api::list_tasks(token) {
            // Don't replace tasks with empty list (could be a temporary API issue)
            if tasks.is_empty() && !self.cached_tasks.is_empty() {
                return;
            }

            // Update base_elapsed for non-running tasks only
            // Running task's base_elapsed is managed by mark_synced()
            for t in &tasks {
                if self.running_task_id != Some(t.id) {
                    self.base_elapsed.insert(t.id, t.elapsed_secs);
                }
            }

            self.cached_tasks = tasks
                .into_iter()
                .map(|t| Task {
                    id: t.id,
                    name: t.name,
                    budget_secs: t.budget_secs,
                    elapsed_secs: 0, // We use base_elapsed map instead
                    running: false,  // We track running state locally
                    project_id: t.project_id,
                    project_name: t.project_name,
                })
                .collect();
            self.last_sync_at = Utc::now();
        }
    }

    /// Start a timer for a task (calls API + updates local state)
    pub fn start_task(&mut self, task_id: i64, token: &Option<String>) -> Result<(), String> {
        // Stop current task first if one is running
        if let Some(current_id) = self.running_task_id {
            if current_id != task_id {
                self.stop_current(token)?;
            } else {
                return Ok(()); // Already running this task
            }
        }

        // Track locally - sync happens periodically or on stop
        let now = Utc::now();
        self.running_task_id = Some(task_id);
        self.session_started_at = Some(now);
        self.last_slot_sync_at = Some(now);
        self.timer_started_at = Some(now);
        self.last_task_id = Some(task_id);
        Ok(())
    }

    /// Resume a task after idle, adding the idle duration as work time
    pub fn resume_with_idle_time(
        &mut self,
        task_id: i64,
        idle_secs: i64,
        _token: &Option<String>,
    ) -> Result<(), String> {
        // Add the idle duration to this task's base elapsed
        let base = self.base_elapsed.entry(task_id).or_insert(0);
        *base += idle_secs;

        // Resume tracking locally
        let now = Utc::now();
        self.running_task_id = Some(task_id);
        self.session_started_at = Some(now);
        self.last_slot_sync_at = Some(now);
        self.timer_started_at = Some(now);
        self.last_task_id = Some(task_id);
        Ok(())
    }

    /// Stop the currently running timer locally (no API call).
    ///
    /// Returns information about the stopped timer, or `None` if no timer was running.
    /// Includes protection against clock anomalies.
    pub fn stop_current_local(&mut self) -> Option<StopInfo> {
        let task_id = self.running_task_id?;
        let started_at = self.timer_started_at?;

        let raw_elapsed = (Utc::now() - started_at).num_seconds();
        // Apply clock protections
        let elapsed = if raw_elapsed < 0 {
            eprintln!("[timer] Warning: Negative elapsed time detected on stop, using 0");
            0
        } else if raw_elapsed > Self::MAX_SESSION_SECS {
            eprintln!(
                "[timer] Warning: Excessive elapsed time {} detected on stop, capping at {}",
                raw_elapsed, Self::MAX_SESSION_SECS
            );
            Self::MAX_SESSION_SECS
        } else {
            raw_elapsed
        };

        let base = self.base_elapsed.entry(task_id).or_insert(0);
        // Use saturating_add to prevent overflow
        *base = base.saturating_add(elapsed).min(Self::MAX_TOTAL_ELAPSED_SECS);

        self.running_task_id = None;
        self.timer_started_at = None;
        self.last_task_id = Some(task_id);

        Some(StopInfo {
            task_id,
            started_at,
            elapsed_secs: elapsed,
        })
    }

    /// Stop the currently running timer and sync to backend.
    ///
    /// # Errors
    ///
    /// Returns an error if synchronization fails, though timing data is preserved locally.
    pub fn stop_current(&mut self, token: &Option<String>) -> Result<(), String> {
        // Capture session info before stopping
        let session_start = self.session_started_at;
        let last_slot_sync = self.last_slot_sync_at;

        if let Some(info) = self.stop_current_local() {
            // Calculate slot_seconds since last sync (not total elapsed)
            let slot_seconds = if let Some(last_sync) = last_slot_sync {
                let now = Utc::now();
                let slot_secs = (now - last_sync).num_seconds();
                if slot_secs < 0 { 0 } else { slot_secs }
            } else {
                info.elapsed_secs
            };

            // Sync to backend if there's meaningful elapsed time
            if slot_seconds > 0 {
                let session_start_str = session_start
                    .unwrap_or(info.started_at)
                    .to_rfc3339();
                let slot_end_str = Utc::now().to_rfc3339();

                match api::sync_time(
                    info.task_id,
                    slot_seconds,
                    &session_start_str,
                    &slot_end_str,
                    true, // is_final = true for stop
                    token,
                ) {
                    Ok(_response) => {
                        println!(
                            "[timer] Stop synced {} slot seconds for task {}",
                            slot_seconds, info.task_id
                        );
                    }
                    Err(e) => {
                        eprintln!("[timer] Failed to sync stop: {e}");
                    }
                }
            }
        }

        // Clear session tracking
        self.session_started_at = None;
        self.last_slot_sync_at = None;

        Ok(())
    }

    /// Check if midnight has passed and reset timers if so.
    ///
    /// This method compares the current local date with the stored `current_day`.
    /// If the day has changed (midnight crossed), it will:
    /// 1. Stop any running timer and sync to backend
    /// 2. Clear all `base_elapsed` values (daily totals)
    /// 3. Update `current_day` to today
    ///
    /// Returns `Some(MidnightResetInfo)` if a reset occurred, `None` otherwise.
    pub fn check_midnight_reset(&mut self, token: &Option<String>) -> Option<MidnightResetInfo> {
        let today = Local::now().date_naive();

        if today == self.current_day {
            return None;
        }

        println!(
            "[midnight] Day changed from {} to {} - performing reset",
            self.current_day, today
        );

        let mut reset_info = MidnightResetInfo {
            synced_task_id: None,
            synced_elapsed: 0,
        };

        // If a timer is running, sync the accumulated time before reset
        if let Some(task_id) = self.running_task_id {
            let now = Utc::now();

            // Calculate slot_seconds since last sync
            let slot_seconds = self.last_slot_sync_at
                .map(|last_sync| {
                    let secs = (now - last_sync).num_seconds();
                    if secs < 0 { 0 } else { secs }
                })
                .unwrap_or_else(|| self.get_total_elapsed(task_id));

            if slot_seconds > 0 {
                let session_start_str = self.session_started_at
                    .or(self.timer_started_at)
                    .map(|t| t.to_rfc3339())
                    .unwrap_or_else(|| now.to_rfc3339());
                let slot_end_str = now.to_rfc3339();

                println!(
                    "[midnight] Syncing {} slot seconds for task {} before reset",
                    slot_seconds, task_id
                );

                match api::sync_time(
                    task_id,
                    slot_seconds,
                    &session_start_str,
                    &slot_end_str,
                    true, // is_final = true
                    token,
                ) {
                    Ok(_) => {
                        println!("[midnight] Sync successful");
                        reset_info.synced_task_id = Some(task_id);
                        reset_info.synced_elapsed = slot_seconds;
                    }
                    Err(e) => {
                        eprintln!("[midnight] Sync failed: {e}");
                    }
                }
            }

            // Stop the timer and clear session tracking
            self.running_task_id = None;
            self.timer_started_at = None;
            self.session_started_at = None;
            self.last_slot_sync_at = None;
            self.last_task_id = Some(task_id);
        }

        // Clear all elapsed times - new day starts fresh
        self.base_elapsed.clear();
        self.last_synced_elapsed.clear();

        // Update to today
        self.current_day = today;

        println!("[midnight] Reset complete - all elapsed times cleared");

        Some(reset_info)
    }

    /// Returns the current tracking day.
    #[must_use]
    #[inline]
    pub fn current_day(&self) -> NaiveDate {
        self.current_day
    }

    /// Sync the current time slot to backend (called periodically).
    /// Returns the number of slot seconds synced, or None if no timer is running.
    pub fn sync_slot(&mut self, token: &Option<String>) -> Option<i64> {
        let task_id = self.running_task_id?;
        let session_start = self.session_started_at?;
        let last_slot_sync = self.last_slot_sync_at?;

        let now = Utc::now();
        let slot_seconds = (now - last_slot_sync).num_seconds();

        // Skip if no meaningful time to sync
        if slot_seconds <= 0 {
            return Some(0);
        }

        let session_start_str = session_start.to_rfc3339();
        let slot_end_str = now.to_rfc3339();

        match api::sync_time(
            task_id,
            slot_seconds,
            &session_start_str,
            &slot_end_str,
            false, // is_final = false for periodic sync
            token,
        ) {
            Ok(_response) => {
                println!(
                    "[sync] Synced {} slot seconds for task {}",
                    slot_seconds, task_id
                );
                // Update last_slot_sync_at to now
                self.last_slot_sync_at = Some(now);
                // Update the synced elapsed tracking
                let total = self.get_total_elapsed(task_id);
                self.last_synced_elapsed.insert(task_id, total);
                Some(slot_seconds)
            }
            Err(e) => {
                eprintln!("[sync] Failed to sync slot: {e}");
                None
            }
        }
    }
}

/// Spawns a background thread that syncs with the external API every 10 minutes
pub fn spawn_sync_thread(
    timer_state: Arc<Mutex<TimerStateInner>>,
    auth_state: Arc<Mutex<AuthTokenState>>,
) {
    std::thread::spawn(move || {
        // Initial sync
        {
            let token = get_token(&auth_state);
            if let Ok(mut s) = timer_state.lock() {
                s.sync_from_api(&token);
                println!(
                    "[sync] Initial sync complete, {} tasks loaded",
                    s.cached_tasks.len()
                );
            }
        }

        loop {
            std::thread::sleep(Duration::from_secs(SYNC_INTERVAL_SECS));

            let token = get_token(&auth_state);
            if let Ok(mut s) = timer_state.lock() {
                // First, sync the current time slot to backend if timer is running
                if s.is_running() {
                    if let Some(slot_secs) = s.sync_slot(&token) {
                        println!("[sync] Slot sync complete: {} seconds", slot_secs);
                    }
                }

                // Then sync task list from API
                println!("[sync] Syncing task list from API...");
                s.sync_from_api(&token);
                println!("[sync] Task sync complete");
            }
        }
    });
}

/// Helper to read the current auth token
fn get_token(auth_state: &Arc<Mutex<AuthTokenState>>) -> Option<String> {
    auth_state.lock().ok().and_then(|s| s.access_token.clone())
}
