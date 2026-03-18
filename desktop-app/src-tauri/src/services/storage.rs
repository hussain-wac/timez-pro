use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TimeStamp {
    pub timestamp: String,
    pub elapsed_secs: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LocalTimeEntry {
    pub task_id: i64,
    pub client_started_at: String,
    pub client_stopped_at: Option<String>,
    pub synced: bool,
    pub timestamps: Vec<TimeStamp>,
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
            if let Some(entry) = store
                .entries
                .iter_mut()
                .find(|e| e.task_id == task_id && !e.synced)
            {
                entry.client_started_at = started_at;
                entry.client_stopped_at = None;
                entry.timestamps.clear();
            } else {
                store.entries.push(LocalTimeEntry {
                    task_id,
                    client_started_at: started_at,
                    client_stopped_at: None,
                    synced: false,
                    timestamps: vec![],
                });
            }
            drop(store);
            self.save();
        }
    }

    pub fn set_running(&self, task_id: i64, started_at: String) {
        if let Ok(mut store) = self.inner.lock() {
            if let Some(entry) = store
                .entries
                .iter_mut()
                .find(|e| e.task_id == task_id && !e.synced)
            {
                entry.client_started_at = started_at;
                entry.client_stopped_at = None;
                entry.timestamps.clear();
            } else {
                store.entries.push(LocalTimeEntry {
                    task_id,
                    client_started_at: started_at,
                    client_stopped_at: None,
                    synced: false,
                    timestamps: vec![],
                });
            }
            store.was_running = true;
            store.last_running_task_id = Some(task_id);
            drop(store);
            self.save();
        }
    }

    pub fn set_stopped(&self, task_id: i64) {
        if let Ok(mut store) = self.inner.lock() {
            if let Some(entry) = store
                .entries
                .iter_mut()
                .find(|e| e.task_id == task_id && !e.synced)
            {
                entry.client_stopped_at = Some(Utc::now().to_rfc3339());
            }
            store.was_running = false;
            drop(store);
            self.save();
        }
    }

    pub fn add_timestamp(&self, task_id: i64, elapsed: i64) {
        if let Ok(mut store) = self.inner.lock() {
            if let Some(entry) = store
                .entries
                .iter_mut()
                .find(|e| e.task_id == task_id && !e.synced)
            {
                entry.timestamps.push(TimeStamp {
                    timestamp: Utc::now().to_rfc3339(),
                    elapsed_secs: elapsed,
                });
            }
            drop(store);
            self.save();
        }
    }

    pub fn get_entry(&self, task_id: i64) -> Option<LocalTimeEntry> {
        let store = self.inner.lock().ok()?;
        store
            .entries
            .iter()
            .find(|e| e.task_id == task_id && !e.synced)
            .cloned()
    }

    pub fn get_unsynced_entries(&self) -> Vec<LocalTimeEntry> {
        let store = self.inner.lock().ok();
        store
            .map(|s| s.entries.iter().filter(|e| !e.synced).cloned().collect())
            .unwrap_or_default()
    }

    pub fn mark_synced(&self, task_id: i64) {
        if let Ok(mut store) = self.inner.lock() {
            if let Some(entry) = store.entries.iter_mut().find(|e| e.task_id == task_id) {
                entry.synced = true;
            }
            drop(store);
            self.save();
        }
    }

    pub fn was_running(&self) -> bool {
        self.inner
            .lock()
            .ok()
            .map(|s| s.was_running)
            .unwrap_or(false)
    }

    pub fn get_last_running_task_id(&self) -> Option<i64> {
        self.inner.lock().ok().and_then(|s| s.last_running_task_id)
    }

    pub fn clear_running_state(&self) {
        if let Ok(mut store) = self.inner.lock() {
            store.was_running = false;
            store.last_running_task_id = None;
            drop(store);
            self.save();
        }
    }

    pub fn get_auth_token(&self) -> Option<String> {
        self.inner.lock().ok().and_then(|s| s.auth_token.clone())
    }

    pub fn set_auth_token(&self, token: String) {
        if let Ok(mut store) = self.inner.lock() {
            store.auth_token = Some(token);
            drop(store);
            self.save();
        }
    }
}

use chrono::Utc;
