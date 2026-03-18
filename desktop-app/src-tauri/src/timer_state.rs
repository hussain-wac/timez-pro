use std::sync::{Mutex, PoisonError};
use std::time::Duration;

use chrono::Utc;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_notification::NotificationExt;

use crate::api;
use crate::api::AuthToken;
use crate::local_store::LocalTimeStorage;
use crate::models::Task;

/// Send a desktop notification
fn send_notification(app_handle: &AppHandle, title: &str, body: &str) {
    if let Err(e) = app_handle
        .notification()
        .builder()
        .title(title)
        .body(body)
        .show()
    {
        eprintln!("[notification] Failed to send: {}", e);
    }
}

/// Format seconds into a human-readable string like "2 min 30 sec" or "1 hr 5 min"
fn format_duration(secs: i64) -> String {
    if secs < 60 {
        format!("{} sec", secs)
    } else if secs < 3600 {
        let mins = secs / 60;
        let remaining_secs = secs % 60;
        if remaining_secs > 0 {
            format!("{} min {} sec", mins, remaining_secs)
        } else {
            format!("{} min", mins)
        }
    } else {
        let hrs = secs / 3600;
        let mins = (secs % 3600) / 60;
        if mins > 0 {
            format!("{} hr {} min", hrs, mins)
        } else {
            format!("{} hr", hrs)
        }
    }
}

/// Local timer state that tracks everything without hitting the external API.
/// The external API is only called every SYNC_INTERVAL for persistence.
pub struct TimerStateInner {
    /// Cached task list from the last API sync
    pub cached_tasks: Vec<Task>,
    /// Currently running task id
    pub running_task_id: Option<i64>,
    /// When the current timer was started (local clock)
    pub timer_started_at: Option<chrono::DateTime<Utc>>,
    /// When the current timer was started (ISO8601 string for API)
    pub client_started_at: Option<String>,
    /// Last task id we ran (used for resume)
    pub last_task_id: Option<i64>,
    /// Last time we synced with the external API
    pub last_sync_at: chrono::DateTime<Utc>,
    /// Elapsed seconds accumulated before current run (from API summary)
    pub base_elapsed: std::collections::HashMap<i64, i64>,
}

pub type TimerState = Mutex<TimerStateInner>;

const SYNC_INTERVAL_SECS: u64 = 30; // 30 seconds

/// Helper trait to recover from poisoned mutex
trait RecoverableMutex<T> {
    fn lock_or_recover(&self) -> Result<std::sync::MutexGuard<'_, T>, String>;
}

impl<T> RecoverableMutex<T> for Mutex<T> {
    fn lock_or_recover(&self) -> Result<std::sync::MutexGuard<'_, T>, String> {
        self.lock().map_err(|e: PoisonError<_>| {
            eprintln!("[CRITICAL] Mutex poisoned, recovering: {}", e);
            // In a real app, you might want to clear the poison and recover
            // For now, we'll return the error but still try to use the data
            format!("Mutex poisoned: {}", e)
        }).or_else(|_| {
            // Try to recover by clearing the poison
            self.clear_poison();
            self.lock().map_err(|e| format!("Failed to recover mutex: {}", e))
        })
    }
}

impl TimerStateInner {
    pub fn new() -> Self {
        Self {
            cached_tasks: vec![],
            running_task_id: None,
            timer_started_at: None,
            client_started_at: None,
            last_task_id: None,
            last_sync_at: chrono::DateTime::<Utc>::MIN_UTC,
            base_elapsed: std::collections::HashMap::new(),
        }
    }

    /// Get tasks with locally computed elapsed time
    pub fn get_tasks(&self) -> Vec<Task> {
        let now = Utc::now();
        let mut out = Vec::with_capacity(self.cached_tasks.len());
        let running_id = self.running_task_id;
        let live_elapsed = if let (Some(_), Some(started)) = (running_id, self.timer_started_at) {
            (now - started).num_seconds().max(0)
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
            out.push(Task {
                id: t.id,
                name: t.name.clone(),
                budget_secs: t.budget_secs,
                elapsed_secs: base + if is_running { live_elapsed } else { 0 },
                running: is_running,
            });
        }

        out
    }

    /// Sync cached data from the external API
    pub fn sync_from_api(&mut self, token: &Option<String>) {
        if let Ok(tasks) = api::list_tasks(token) {
            // Rebuild base elapsed from API to avoid stale entries
            let mut base_elapsed = std::collections::HashMap::with_capacity(tasks.len());
            for t in &tasks {
                base_elapsed.insert(t.id, t.elapsed_secs);
            }

            self.base_elapsed = base_elapsed;
            self.cached_tasks = tasks
                .into_iter()
                .map(|t| Task {
                    id: t.id,
                    name: t.name,
                    budget_secs: t.budget_secs,
                    elapsed_secs: 0, // We use base_elapsed map instead
                    running: false,  // We track running state locally
                })
                .collect();
            self.last_sync_at = Utc::now();
        }
    }

    /// Start a timer for a task (local only - no API call)
    pub fn start_task(
        &mut self,
        task_id: i64,
        local_store: &LocalTimeStorage,
    ) -> Result<(), String> {
        if let Some(current_id) = self.running_task_id {
            if current_id != task_id {
                self.stop_current_local(local_store);
            } else {
                return Ok(());
            }
        }

        let now = Utc::now();
        let client_started = now.to_rfc3339();

        local_store.set_running(task_id, client_started.clone());

        self.running_task_id = Some(task_id);
        self.timer_started_at = Some(now);
        self.client_started_at = Some(client_started);
        self.last_task_id = Some(task_id);
        Ok(())
    }

    /// Resume a task after idle, adding the idle duration as work time (local only)
    pub fn resume_with_idle_time(
        &mut self,
        task_id: i64,
        idle_secs: i64,
        local_store: &LocalTimeStorage,
    ) -> Result<(), String> {
        let base = self.base_elapsed.entry(task_id).or_insert(0);
        *base += idle_secs;

        let now = Utc::now();
        let client_started = now.to_rfc3339();

        local_store.start_timer(task_id, client_started.clone());

        self.running_task_id = Some(task_id);
        self.timer_started_at = Some(now);
        self.client_started_at = Some(client_started);
        self.last_task_id = Some(task_id);
        Ok(())
    }

    /// Stop the currently running timer locally (no API call)
    pub fn stop_current_local(&mut self, local_store: &LocalTimeStorage) -> Option<(i64, String)> {
        if let Some(task_id) = self.running_task_id {
            if let Some(started) = self.timer_started_at {
                let elapsed = (Utc::now() - started).num_seconds().max(0);
                let base = self.base_elapsed.entry(task_id).or_insert(0);
                *base += elapsed;
            }
            let client_stopped = Utc::now().to_rfc3339();

            local_store.set_stopped(task_id);

            self.running_task_id = None;
            self.timer_started_at = None;
            self.last_task_id = Some(task_id);
            return Some((task_id, client_stopped));
        }
        None
    }

    /// Stop the currently running timer and sync to backend
    pub fn stop_current(
        &mut self,
        token: &Option<String>,
        local_store: &LocalTimeStorage,
    ) -> Result<(), String> {
        if let Some(task_id) = self.running_task_id {
            // Calculate total elapsed before stopping
            let live_elapsed = self
                .timer_started_at
                .map(|started| (Utc::now() - started).num_seconds().max(0))
                .unwrap_or(0);
            let base_elapsed = self.base_elapsed.get(&task_id).copied().unwrap_or(0);
            let total_elapsed = base_elapsed + live_elapsed;
            let client_started = self.client_started_at.clone()
                .unwrap_or_else(|| Utc::now().to_rfc3339());
            let client_stopped = Utc::now().to_rfc3339();

            // Stop locally first
            self.stop_current_local(local_store);

            // Sync to backend if we have time to sync
            if total_elapsed > 0 {
                match api::sync_time(
                    task_id,
                    total_elapsed,
                    &client_started,
                    Some(&client_stopped),
                    token,
                ) {
                    Ok(response) => {
                        eprintln!(
                            "[timer] Stop synced: task_id={}, duration={:?}, is_synced={}",
                            response.task_id, response.duration, response.is_synced
                        );
                        local_store.mark_synced(task_id, total_elapsed);
                    }
                    Err(e) => {
                        eprintln!("[timer] Failed to sync stop: {}", e);
                        // Don't return error - local stop already succeeded
                    }
                }
            }
        }
        Ok(())
    }

    /// Extract sync data without holding lock during network I/O
    pub fn get_sync_data(&self) -> Option<SyncData> {
        self.running_task_id.map(|task_id| {
            let live_elapsed = self
                .timer_started_at
                .map(|started| (Utc::now() - started).num_seconds().max(0))
                .unwrap_or(0);
            let base_elapsed = self.base_elapsed.get(&task_id).copied().unwrap_or(0);
            SyncData {
                task_id,
                total_elapsed: base_elapsed + live_elapsed,
                client_started_at: self.client_started_at.clone()
                    .unwrap_or_else(|| Utc::now().to_rfc3339()),
            }
        })
    }
}

/// Data needed for sync, extracted from state
#[derive(Clone)]
pub struct SyncData {
    pub task_id: i64,
    pub total_elapsed: i64,
    pub client_started_at: String,
}

/// Spawns a background thread that syncs with the external API every 30 seconds
pub fn spawn_sync_thread(app_handle: AppHandle) {
    std::thread::spawn(move || {
        // Initial sync
        {
            let token = get_token(&app_handle);
            let state = app_handle.state::<TimerState>();
            match state.inner().lock_or_recover() {
                Ok(mut s) => {
                    s.sync_from_api(&token);
                    println!(
                        "[sync] Initial sync complete, {} tasks loaded",
                        s.cached_tasks.len()
                    );
                }
                Err(e) => eprintln!("[sync] Initial sync failed: {}", e),
            }
        }

        loop {
            std::thread::sleep(Duration::from_secs(SYNC_INTERVAL_SECS));

            let token = get_token(&app_handle);
            let state = app_handle.state::<TimerState>();
            let local_store = app_handle.state::<LocalTimeStorage>();

            // Check for midnight reset (separate lock scope)
            {
                let now = Utc::now();
                if now.hour() == 0 && now.minute() == 0 {
                    if let Ok(mut s) = state.inner().lock_or_recover() {
                        if s.running_task_id.is_some() {
                            s.stop_current_local(&local_store);
                            let _ = app_handle.emit("midnight-reset", ());
                        }
                    }
                }
            }

            // Emit sync notification
            let _ = app_handle.emit("sync-in-progress", ());

            // Get unsynced entries OUTSIDE the lock
            let entries = local_store.get_unsynced_entries();
            println!("[sync] Found {} unsynced entries", entries.len());

            // Process each entry WITHOUT holding the mutex
            for entry in entries {
                let task_id = entry.task_id;
                let client_started_at = entry.client_started_at.clone();
                let client_stopped_at = entry.client_stopped_at.clone();

                // Get elapsed from the LAST timestamp (cumulative, not delta)
                let total_elapsed: i64 =
                    entry.timestamps.last().map(|t| t.elapsed_secs).unwrap_or(0);

                // Calculate how much NEW time we're syncing
                let new_time_to_sync = total_elapsed - entry.last_synced_elapsed;

                println!(
                    "[sync] Task {} has {} timestamps, total {} secs, new {} secs",
                    task_id,
                    entry.timestamps.len(),
                    total_elapsed,
                    new_time_to_sync
                );

                if total_elapsed > 0 && new_time_to_sync > 0 {
                    // Send desktop notification BEFORE syncing
                    send_notification(
                        &app_handle,
                        "Timez Pro - Syncing",
                        &format!("Syncing {} to server...", format_duration(new_time_to_sync))
                    );

                    // Network I/O happens OUTSIDE mutex lock
                    let result = api::sync_time(
                        task_id,
                        total_elapsed,
                        &client_started_at,
                        client_stopped_at.as_deref(),
                        &token,
                    );

                    match result {
                        Err(e) => {
                            println!("[sync] Error syncing task {}: {}", task_id, e);
                            send_notification(
                                &app_handle,
                                "Timez Pro - Sync Failed",
                                &format!("Failed to sync time: {}", e)
                            );
                        }
                        Ok(response) => {
                            // Handshake confirmed - backend received the data
                            println!(
                                "[sync] Handshake confirmed: task_id={}, backend_duration={:?}, is_synced={}",
                                response.task_id, response.duration, response.is_synced
                            );
                            println!(
                                "[sync] Synced {} seconds for task {} (new: {} seconds)",
                                total_elapsed, task_id, new_time_to_sync
                            );
                            local_store.mark_synced(task_id, total_elapsed);

                            // Send success notification
                            send_notification(
                                &app_handle,
                                "Timez Pro - Synced",
                                &format!("{} synced successfully", format_duration(new_time_to_sync))
                            );

                            // Also emit event for UI update
                            let _ = app_handle.emit(
                                "sync-complete",
                                serde_json::json!({
                                    "task_id": task_id,
                                    "synced_seconds": new_time_to_sync,
                                    "total_seconds": total_elapsed,
                                    "backend_confirmed": response.is_synced
                                }),
                            );
                        }
                    }
                }
            }

            // Clean up completed entries
            local_store.cleanup_completed_entries();

            // Sync from API (separate lock scope, quick operation)
            if let Ok(mut s) = state.inner().lock_or_recover() {
                s.sync_from_api(&token);
                println!("[sync] Sync complete");
            }
        }
    });
}

/// Spawns a thread that records timestamps every 5 seconds
pub fn spawn_timestamp_thread(app_handle: AppHandle) {
    std::thread::spawn(move || {
        loop {
            std::thread::sleep(Duration::from_secs(5));

            let local_store = app_handle.state::<LocalTimeStorage>();
            let timer_state = app_handle.state::<TimerState>();

            // Get running task and elapsed time (short lock)
            let (task_id, elapsed) = match timer_state.inner().lock_or_recover() {
                Ok(s) => {
                    if let Some(id) = s.running_task_id {
                        let elapsed = s
                            .timer_started_at
                            .map(|started| (chrono::Utc::now() - started).num_seconds().max(0))
                            .unwrap_or(0);
                        (Some(id), elapsed)
                    } else {
                        (None, 0)
                    }
                }
                Err(e) => {
                    eprintln!("[timestamp] Failed to acquire lock: {}", e);
                    (None, 0)
                }
            };

            // Record timestamp (outside lock)
            if let Some(id) = task_id {
                local_store.add_timestamp(id, elapsed);
                println!("[timestamp] Recorded: task_id={}, elapsed={}s", id, elapsed);
            }
        }
    });
}

/// Helper to read the current auth token
fn get_token(app_handle: &AppHandle) -> Option<String> {
    let auth = app_handle.state::<AuthToken>();
    match auth.inner().lock_or_recover() {
        Ok(s) => s.access_token.clone(),
        Err(e) => {
            eprintln!("[auth] Failed to get token: {}", e);
            None
        }
    }
}
