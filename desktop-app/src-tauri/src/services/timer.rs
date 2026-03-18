use std::sync::{Arc, Mutex};
use std::time::Duration;

use chrono::Utc;
use tauri::{AppHandle, Emitter, Manager};

use crate::models::Task;
use crate::services::api;
use crate::services::api::AuthToken;
use crate::services::storage::LocalTimeStorage;

const SYNC_INTERVAL_SECS: u64 = 30; // Sync every 30 seconds

pub struct TimerStateInner {
    pub cached_tasks: Vec<Task>,
    pub running_task_id: Option<i64>,
    pub timer_started_at: Option<chrono::DateTime<Utc>>,
    pub client_started_at: Option<String>,
    pub last_task_id: Option<i64>,
    pub last_sync_at: chrono::DateTime<Utc>,
    pub base_elapsed: std::collections::HashMap<i64, i64>,
}

pub type TimerState = Arc<Mutex<TimerStateInner>>;

pub fn new_timer_state() -> TimerState {
    Arc::new(Mutex::new(TimerStateInner::new()))
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

    pub fn sync_from_api(&mut self, token: &Option<String>) {
        if let Ok(tasks) = api::list_tasks(token) {
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
                    elapsed_secs: 0,
                    running: false,
                })
                .collect();
            self.last_sync_at = Utc::now();
        }
    }

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

    pub fn is_tracking(&self) -> bool {
        self.running_task_id.is_some()
    }

    pub fn get_active_task(&self) -> Option<i64> {
        self.running_task_id
    }

    pub fn add_elapsed_time(&mut self, elapsed_secs: i64) {
        if let Some(task_id) = self.running_task_id {
            let base = self.base_elapsed.entry(task_id).or_insert(0);
            *base += elapsed_secs;
        }
    }
}

fn get_token(app_handle: &AppHandle) -> Option<String> {
    let auth = app_handle.state::<AuthToken>();
    auth.lock().ok().and_then(|s| s.access_token.clone())
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

pub fn spawn_sync_thread(app_handle: AppHandle, timer_state: TimerState) {
    std::thread::spawn(move || {
        // Initial sync
        {
            let token = get_token(&app_handle);
            if let Ok(mut s) = timer_state.lock() {
                s.sync_from_api(&token);
                eprintln!(
                    "[sync] Initial sync complete, {} tasks loaded",
                    s.cached_tasks.len()
                );
            }
        }

        loop {
            std::thread::sleep(Duration::from_secs(SYNC_INTERVAL_SECS));

            let token = get_token(&app_handle);
            let local_store = app_handle.state::<LocalTimeStorage>();

            // Check for midnight reset
            let now = Utc::now();
            if now.hour() == 0 && now.minute() == 0 {
                if let Ok(mut s) = timer_state.lock() {
                    if s.running_task_id.is_some() {
                        s.stop_current_local(&local_store);
                        let _ = app_handle.emit("midnight-reset", ());
                    }
                }
            }

            if let Ok(mut s) = timer_state.lock() {
                eprintln!("[sync] Syncing with API...");

                let entries = local_store.get_entries_to_sync();
                eprintln!("[sync] Found {} entries to sync", entries.len());

                for entry in entries {
                    let task_id = entry.task_id;
                    let client_started_at = entry.client_started_at.clone();
                    let client_stopped_at = entry.client_stopped_at.clone();

                    // Get the current elapsed time from the LAST timestamp
                    let current_elapsed: i64 = entry.timestamps.last().map(|t| t.elapsed_secs).unwrap_or(0);

                    // Calculate how much NEW time we're syncing
                    let new_time_to_sync = current_elapsed - entry.last_synced_elapsed;

                    if current_elapsed > 0 && new_time_to_sync > 0 {
                        // Emit notification BEFORE syncing so user sees it
                        let _ = app_handle.emit("sync-in-progress", serde_json::json!({
                            "task_id": task_id,
                            "syncing_seconds": new_time_to_sync,
                            "total_seconds": current_elapsed,
                            "message": format!("Syncing {} to backend", format_duration(new_time_to_sync))
                        }));

                        let result = api::sync_time(
                            task_id,
                            current_elapsed,
                            &client_started_at,
                            client_stopped_at.as_deref(),
                            &token,
                        );

                        if let Err(e) = result {
                            eprintln!("[sync] Error syncing task {}: {}", task_id, e);
                            let _ = app_handle.emit("sync-error", serde_json::json!({
                                "task_id": task_id,
                                "error": e
                            }));
                        } else {
                            eprintln!(
                                "[sync] Synced {} seconds for task {} (new: {} seconds)",
                                current_elapsed, task_id, new_time_to_sync
                            );

                            // Mark as synced with the current elapsed time
                            local_store.mark_synced(task_id, current_elapsed);

                            let _ = app_handle.emit("sync-complete", serde_json::json!({
                                "task_id": task_id,
                                "synced_seconds": new_time_to_sync,
                                "total_seconds": current_elapsed,
                                "message": format!("{} synced successfully", format_duration(new_time_to_sync))
                            }));
                        }
                    }
                }

                // Clean up completed entries periodically
                local_store.cleanup_completed_entries();

                // Refresh tasks from API
                s.sync_from_api(&token);
                eprintln!("[sync] Sync complete");
            }
        }
    });
}

pub fn spawn_timestamp_thread(app_handle: AppHandle, timer_state: TimerState) {
    std::thread::spawn(move || loop {
        std::thread::sleep(Duration::from_secs(5));

        let local_store = app_handle.state::<LocalTimeStorage>();

        let (task_id, elapsed) = {
            if let Ok(s) = timer_state.lock() {
                if let Some(id) = s.running_task_id {
                    let elapsed = s
                        .timer_started_at
                        .map(|started| (chrono::Utc::now() - started).num_seconds().max(0))
                        .unwrap_or(0);
                    (Some(id), elapsed)
                } else {
                    (None, 0)
                }
            } else {
                (None, 0)
            }
        };

        if let Some(id) = task_id {
            local_store.add_timestamp(id, elapsed);
            eprintln!("[timestamp] Recorded: task_id={}, elapsed={}s", id, elapsed);
        }
    });
}
