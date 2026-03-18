use std::sync::{Arc, Mutex};
use std::time::Duration;

use chrono::Utc;

use crate::api;
use crate::api::AuthTokenState;
use crate::models::Task;

/// Local timer state that tracks everything without hitting the external API.
/// The external API is only called every SYNC_INTERVAL for persistence.
pub struct TimerStateInner {
    /// Cached task list from the last API sync
    pub cached_tasks: Vec<Task>,
    /// Currently running task id
    pub running_task_id: Option<i64>,
    /// When the current timer was started (local clock)
    pub timer_started_at: Option<chrono::DateTime<Utc>>,
    /// Last task id we ran (used for resume)
    pub last_task_id: Option<i64>,
    /// Last time we synced with the external API
    pub last_sync_at: chrono::DateTime<Utc>,
    /// Elapsed seconds accumulated before current run (from API summary)
    pub base_elapsed: std::collections::HashMap<i64, i64>,
    /// Total elapsed sent to backend in last sync (to avoid double-counting)
    pub last_synced_elapsed: std::collections::HashMap<i64, i64>,
}

pub type TimerState = Mutex<TimerStateInner>;

const SYNC_INTERVAL_SECS: u64 = 60; // 1 minute

impl TimerStateInner {
    pub fn new() -> Self {
        Self {
            cached_tasks: vec![],
            running_task_id: None,
            timer_started_at: None,
            last_task_id: None,
            last_sync_at: chrono::DateTime::<Utc>::MIN_UTC,
            base_elapsed: std::collections::HashMap::new(),
            last_synced_elapsed: std::collections::HashMap::new(),
        }
    }

    /// Get total elapsed for a running task (base + live)
    pub fn get_total_elapsed(&self, task_id: i64) -> i64 {
        let base = self.base_elapsed.get(&task_id).copied().unwrap_or(0);
        let live = if self.running_task_id == Some(task_id) {
            self.timer_started_at
                .map(|started| (Utc::now() - started).num_seconds().max(0))
                .unwrap_or(0)
        } else {
            0
        };
        base + live
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
        self.running_task_id = Some(task_id);
        self.timer_started_at = Some(Utc::now());
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
        self.running_task_id = Some(task_id);
        self.timer_started_at = Some(Utc::now());
        self.last_task_id = Some(task_id);
        Ok(())
    }

    /// Stop the currently running timer locally (no API call)
    pub fn stop_current_local(&mut self) -> Option<i64> {
        if let Some(task_id) = self.running_task_id {
            if let Some(started) = self.timer_started_at {
                let elapsed = (Utc::now() - started).num_seconds().max(0);
                let base = self.base_elapsed.entry(task_id).or_insert(0);
                *base += elapsed;
            }
            self.running_task_id = None;
            self.timer_started_at = None;
            self.last_task_id = Some(task_id);
            return Some(task_id);
        }
        None
    }

    /// Stop the currently running timer and sync to backend
    pub fn stop_current(&mut self, token: &Option<String>) -> Result<(), String> {
        if let Some(task_id) = self.stop_current_local() {
            // Calculate elapsed and sync to backend
            if let Some(started_at) = self.timer_started_at {
                let elapsed = (chrono::Utc::now() - started_at).num_seconds().max(0);
                let client_started = started_at.to_rfc3339();
                let client_stopped = chrono::Utc::now().to_rfc3339();

                if elapsed > 0 {
                    match api::sync_time(
                        task_id,
                        elapsed,
                        &client_started,
                        Some(&client_stopped),
                        token,
                    ) {
                        Ok(_response) => {
                            println!("[timer] Stop synced {} seconds for task {}", elapsed, task_id);
                        }
                        Err(e) => {
                            eprintln!("[timer] Failed to sync stop: {}", e);
                        }
                    }
                }
            }
        }
        Ok(())
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
                println!("[sync] Syncing with API...");
                s.sync_from_api(&token);
                println!("[sync] Sync complete");
            }
        }
    });
}

/// Helper to read the current auth token
fn get_token(auth_state: &Arc<Mutex<AuthTokenState>>) -> Option<String> {
    auth_state.lock().ok().and_then(|s| s.access_token.clone())
}
