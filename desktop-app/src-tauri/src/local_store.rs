use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;

/// Maximum number of timestamps to retain per entry.
/// This prevents unbounded memory growth for long-running timers.
const MAX_TIMESTAMPS_PER_ENTRY: usize = 200;

/// Represents a single timestamp recording for crash recovery.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TimeStamp {
    pub timestamp: String,
    pub elapsed_secs: i64,
}

/// A local time entry that may or may not be synced to the server.
///
/// This struct supports offline operation by storing time entries locally
/// and syncing them when connectivity is available.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LocalTimeEntry {
    pub task_id: i64,
    pub client_started_at: String,
    pub client_stopped_at: Option<String>,
    pub synced: bool,
    /// Track what was last synced to avoid re-syncing same data
    pub last_synced_elapsed: i64,
    pub timestamps: Vec<TimeStamp>,
}

impl LocalTimeEntry {
    /// Returns `true` if this entry has unsynced time.
    #[must_use]
    #[inline]
    pub fn has_unsynced_time(&self) -> bool {
        if self.timestamps.is_empty() {
            return false;
        }
        let current_elapsed = self.timestamps.last().map_or(0, |t| t.elapsed_secs);
        current_elapsed > self.last_synced_elapsed || self.client_stopped_at.is_some()
    }

    /// Returns `true` if the timer is still running (not stopped).
    #[must_use]
    #[inline]
    pub fn is_running(&self) -> bool {
        self.client_stopped_at.is_none()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LocalTimeStore {
    pub entries: Vec<LocalTimeEntry>,
    pub last_sync_at: Option<String>,
    pub was_running: bool,
    pub last_running_task_id: Option<i64>,
    pub auth_token: Option<String>,
}

pub struct LocalTimeStorage {
    inner: Mutex<LocalTimeStore>,
    path: PathBuf,
}

impl LocalTimeStorage {
    pub fn new(app_data_dir: PathBuf) -> Self {
        let path = app_data_dir.join("time_entries.json");
        let store = if path.exists() {
            fs::read_to_string(&path)
                .ok()
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or_default()
        } else {
            LocalTimeStore::default()
        };

        Self {
            inner: Mutex::new(store),
            path,
        }
    }

    fn save(&self) {
        if let Ok(store) = self.inner.lock() {
            if let Ok(json) = serde_json::to_string_pretty(&*store) {
                if let Some(parent) = self.path.parent() {
                    let _ = fs::create_dir_all(parent);
                }
                let _ = fs::write(&self.path, json);
            }
        }
    }

    pub fn start_timer(&self, task_id: i64, started_at: String) {
        if let Ok(mut store) = self.inner.lock() {
            // Check if there's an existing entry for this task (synced or not)
            if let Some(entry) = store
                .entries
                .iter_mut()
                .find(|e| e.task_id == task_id && e.client_stopped_at.is_none())
            {
                // Reuse existing running entry
                entry.client_started_at = started_at;
                entry.synced = false;
            } else {
                // Create new entry
                store.entries.push(LocalTimeEntry {
                    task_id,
                    client_started_at: started_at,
                    client_stopped_at: None,
                    synced: false,
                    last_synced_elapsed: 0,
                    timestamps: vec![],
                });
            }
            store.was_running = true;
            store.last_running_task_id = Some(task_id);
        }
        self.save();
    }

    pub fn add_timestamp(&self, task_id: i64, elapsed_secs: i64) {
        if let Ok(mut store) = self.inner.lock() {
            // Find entry for this task that is still running (no client_stopped_at)
            if let Some(entry) = store
                .entries
                .iter_mut()
                .find(|e| e.task_id == task_id && e.client_stopped_at.is_none())
            {
                entry.timestamps.push(TimeStamp {
                    timestamp: chrono::Utc::now().to_rfc3339(),
                    elapsed_secs,
                });
                // Keep only last N timestamps to prevent unbounded growth
                if entry.timestamps.len() > MAX_TIMESTAMPS_PER_ENTRY {
                    entry.timestamps.remove(0);
                }
            }
        }
        self.save();
    }

    pub fn stop_timer(&self, task_id: i64, stopped_at: String) {
        if let Ok(mut store) = self.inner.lock() {
            if let Some(entry) = store
                .entries
                .iter_mut()
                .find(|e| e.task_id == task_id && e.client_stopped_at.is_none())
            {
                entry.client_stopped_at = Some(stopped_at);
                entry.synced = false; // Mark as needing sync
            }
            store.was_running = false;
        }
        self.save();
    }

    /// Mark entry as synced and record the elapsed time that was synced.
    /// For stopped entries, remove them. For running entries, keep them but update last_synced_elapsed.
    pub fn mark_synced(&self, task_id: i64, synced_elapsed: i64) {
        if let Ok(mut store) = self.inner.lock() {
            if let Some(entry) = store
                .entries
                .iter_mut()
                .find(|e| e.task_id == task_id)
            {
                if entry.client_stopped_at.is_some() {
                    // Timer was stopped - mark as fully synced
                    entry.synced = true;
                    entry.last_synced_elapsed = synced_elapsed;
                } else {
                    // Timer still running - update last synced but keep entry active
                    entry.last_synced_elapsed = synced_elapsed;
                    entry.synced = false; // Keep as unsynced so we continue syncing
                }
            }
            // Only clear running state if the timer was actually stopped
            if store.entries.iter().any(|e| e.task_id == task_id && e.client_stopped_at.is_some()) {
                store.was_running = false;
                store.last_running_task_id = None;
            }
        }
        self.save();
    }

    /// Clean up old synced entries that are completed (stopped and synced)
    pub fn cleanup_completed_entries(&self) {
        if let Ok(mut store) = self.inner.lock() {
            store.entries.retain(|e| {
                // Keep if: not synced OR still running (no stopped_at)
                !e.synced || e.client_stopped_at.is_none()
            });
        }
        self.save();
    }

    pub fn set_running(&self, task_id: i64, started_at: String) {
        self.start_timer(task_id, started_at);
    }

    pub fn set_stopped(&self, task_id: i64) {
        if let Ok(mut store) = self.inner.lock() {
            store.was_running = false;
            if let Some(entry) = store
                .entries
                .iter_mut()
                .find(|e| e.task_id == task_id && e.client_stopped_at.is_none())
            {
                entry.client_stopped_at = Some(chrono::Utc::now().to_rfc3339());
                entry.synced = false;
            }
        }
        self.save();
    }

    pub fn was_running(&self) -> bool {
        if let Ok(store) = self.inner.lock() {
            store.was_running
        } else {
            false
        }
    }

    pub fn get_last_running_task_id(&self) -> Option<i64> {
        if let Ok(store) = self.inner.lock() {
            store.last_running_task_id
        } else {
            None
        }
    }

    pub fn clear_running_state(&self) {
        if let Ok(mut store) = self.inner.lock() {
            store.was_running = false;
            store.last_running_task_id = None;
        }
        self.save();
    }

    pub fn set_auth_token(&self, token: String) {
        if let Ok(mut store) = self.inner.lock() {
            store.auth_token = Some(token);
        }
        self.save();
    }

    pub fn get_auth_token(&self) -> Option<String> {
        if let Ok(store) = self.inner.lock() {
            store.auth_token.clone()
        } else {
            None
        }
    }

    pub fn clear_auth_token(&self) {
        if let Ok(mut store) = self.inner.lock() {
            store.auth_token = None;
        }
        self.save();
    }

    /// Get entries that need syncing (have new elapsed time since last sync).
    ///
    /// Returns entries that either:
    /// - Have more elapsed time than was last synced
    /// - Have been stopped but not yet synced
    #[must_use]
    pub fn get_entries_to_sync(&self) -> Vec<LocalTimeEntry> {
        match self.inner.lock() {
            Ok(store) => store
                .entries
                .iter()
                .filter(|e| e.has_unsynced_time())
                .cloned()
                .collect(),
            Err(poison_err) => {
                eprintln!("[local_store] Mutex poisoned in get_entries_to_sync, recovering");
                poison_err
                    .into_inner()
                    .entries
                    .iter()
                    .filter(|e| e.has_unsynced_time())
                    .cloned()
                    .collect()
            }
        }
    }

    /// Alias for `get_entries_to_sync` for backward compatibility.
    #[must_use]
    #[inline]
    pub fn get_unsynced_entries(&self) -> Vec<LocalTimeEntry> {
        self.get_entries_to_sync()
    }

    pub fn get_last_running_entry(&self) -> Option<LocalTimeEntry> {
        if let Ok(store) = self.inner.lock() {
            store
                .entries
                .iter()
                .rev()
                .find(|e| e.client_stopped_at.is_none())
                .cloned()
        } else {
            None
        }
    }

    pub fn get_entry(&self, task_id: i64) -> Option<LocalTimeEntry> {
        if let Ok(store) = self.inner.lock() {
            store
                .entries
                .iter()
                .find(|e| e.task_id == task_id && e.client_stopped_at.is_none())
                .cloned()
        } else {
            None
        }
    }

    pub fn update_last_sync(&self) {
        if let Ok(mut store) = self.inner.lock() {
            store.last_sync_at = Some(chrono::Utc::now().to_rfc3339());
        }
        self.save();
    }

    pub fn get_last_sync_at(&self) -> Option<String> {
        if let Ok(store) = self.inner.lock() {
            store.last_sync_at.clone()
        } else {
            None
        }
    }

    pub fn clear(&self) {
        if let Ok(mut store) = self.inner.lock() {
            store.entries.clear();
            store.last_sync_at = None;
        }
        self.save();
    }
}
