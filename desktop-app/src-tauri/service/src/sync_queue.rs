//! Persistent sync queue for time entries.
//!
//! This module provides a disk-backed queue for sync operations that ensures
//! no time data is lost during network failures. Failed syncs are automatically
//! retried with exponential backoff.

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Maximum retry attempts before giving up on a sync entry
const MAX_RETRY_ATTEMPTS: u32 = 10;

/// Base delay for exponential backoff (seconds)
const BASE_RETRY_DELAY_SECS: u64 = 5;

/// Maximum delay cap for exponential backoff (seconds) - 5 minutes
const MAX_RETRY_DELAY_SECS: u64 = 300;

/// A pending sync operation that needs to be sent to the server.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingSyncEntry {
    /// Task ID to sync
    pub task_id: i64,
    /// Total elapsed seconds to sync
    pub elapsed_seconds: i64,
    /// When the client timer started (RFC3339 format)
    pub client_started_at: String,
    /// When the client timer stopped (RFC3339 format), None if still running
    pub client_stopped_at: Option<String>,
    /// Number of retry attempts made
    pub retry_count: u32,
    /// When this entry was first queued
    pub queued_at: DateTime<Utc>,
    /// When the last retry attempt was made
    pub last_attempt_at: Option<DateTime<Utc>>,
    /// Last error message (if any)
    pub last_error: Option<String>,
}

impl PendingSyncEntry {
    /// Creates a new pending sync entry.
    pub fn new(
        task_id: i64,
        elapsed_seconds: i64,
        client_started_at: String,
        client_stopped_at: Option<String>,
    ) -> Self {
        Self {
            task_id,
            elapsed_seconds,
            client_started_at,
            client_stopped_at,
            retry_count: 0,
            queued_at: Utc::now(),
            last_attempt_at: None,
            last_error: None,
        }
    }

    /// Returns true if this entry should be retried now based on exponential backoff.
    pub fn should_retry(&self) -> bool {
        if self.retry_count >= MAX_RETRY_ATTEMPTS {
            return false;
        }

        match self.last_attempt_at {
            None => true, // Never attempted, should try now
            Some(last_attempt) => {
                let delay_secs = self.get_backoff_delay_secs();
                let next_attempt_at = last_attempt + chrono::Duration::seconds(delay_secs as i64);
                Utc::now() >= next_attempt_at
            }
        }
    }

    /// Returns the exponential backoff delay in seconds for the current retry count.
    pub fn get_backoff_delay_secs(&self) -> u64 {
        let delay = BASE_RETRY_DELAY_SECS * 2u64.saturating_pow(self.retry_count);
        delay.min(MAX_RETRY_DELAY_SECS)
    }

    /// Returns true if this entry has exceeded max retries.
    pub fn is_exhausted(&self) -> bool {
        self.retry_count >= MAX_RETRY_ATTEMPTS
    }

    /// Records a failed attempt with an error message.
    pub fn record_failure(&mut self, error: String) {
        self.retry_count += 1;
        self.last_attempt_at = Some(Utc::now());
        self.last_error = Some(error);
    }

    /// Marks this entry as having been attempted now (for success case before removal).
    pub fn record_attempt(&mut self) {
        self.last_attempt_at = Some(Utc::now());
    }
}

/// Persistent sync queue stored on disk.
#[derive(Debug, Serialize, Deserialize, Default)]
struct SyncQueueStore {
    /// Pending sync entries keyed by task_id
    /// We only keep one pending entry per task (latest state)
    entries: HashMap<i64, PendingSyncEntry>,
    /// Entries that have exceeded max retries (for debugging/manual recovery)
    failed_entries: Vec<PendingSyncEntry>,
}

/// Thread-safe sync queue with disk persistence.
pub struct SyncQueue {
    inner: Mutex<SyncQueueStore>,
    path: PathBuf,
}

impl SyncQueue {
    /// Creates a new sync queue, loading any existing state from disk.
    pub fn new() -> Self {
        let path = Self::get_queue_path();
        let store = Self::load_from_disk(&path);

        if !store.entries.is_empty() {
            eprintln!(
                "[sync_queue] Loaded {} pending entries from disk",
                store.entries.len()
            );
        }
        if !store.failed_entries.is_empty() {
            eprintln!(
                "[sync_queue] WARNING: {} entries have exceeded max retries",
                store.failed_entries.len()
            );
        }

        Self {
            inner: Mutex::new(store),
            path,
        }
    }

    /// Returns the platform-appropriate path for the sync queue file.
    fn get_queue_path() -> PathBuf {
        #[cfg(unix)]
        {
            PathBuf::from("/tmp/timez-sync-queue.json")
        }
        #[cfg(windows)]
        {
            std::env::temp_dir().join("timez-sync-queue.json")
        }
        #[cfg(not(any(unix, windows)))]
        {
            PathBuf::from("timez-sync-queue.json")
        }
    }

    /// Loads the queue from disk, returning default if file doesn't exist or is invalid.
    fn load_from_disk(path: &PathBuf) -> SyncQueueStore {
        if !path.exists() {
            return SyncQueueStore::default();
        }

        match fs::read_to_string(path) {
            Ok(contents) => serde_json::from_str(&contents).unwrap_or_else(|e| {
                eprintln!("[sync_queue] Failed to parse queue file: {}", e);
                SyncQueueStore::default()
            }),
            Err(e) => {
                eprintln!("[sync_queue] Failed to read queue file: {}", e);
                SyncQueueStore::default()
            }
        }
    }

    /// Saves the current queue state to disk.
    fn save(&self) {
        if let Ok(store) = self.inner.lock() {
            match serde_json::to_string_pretty(&*store) {
                Ok(json) => {
                    if let Err(e) = fs::write(&self.path, json) {
                        eprintln!("[sync_queue] Failed to save queue: {}", e);
                    }
                }
                Err(e) => {
                    eprintln!("[sync_queue] Failed to serialize queue: {}", e);
                }
            }
        }
    }

    /// Adds or updates a pending sync entry for a task.
    /// If an entry already exists for this task, it will be updated with the new elapsed time.
    pub fn enqueue(
        &self,
        task_id: i64,
        elapsed_seconds: i64,
        client_started_at: String,
        client_stopped_at: Option<String>,
    ) {
        if let Ok(mut store) = self.inner.lock() {
            let entry = PendingSyncEntry::new(
                task_id,
                elapsed_seconds,
                client_started_at,
                client_stopped_at,
            );

            // Always use the latest elapsed time for a task
            store.entries.insert(task_id, entry);

            eprintln!(
                "[sync_queue] Enqueued: task_id={}, elapsed={}",
                task_id, elapsed_seconds
            );
        }
        self.save();
    }

    /// Returns entries that are ready to be retried.
    pub fn get_entries_to_retry(&self) -> Vec<PendingSyncEntry> {
        match self.inner.lock() {
            Ok(store) => store
                .entries
                .values()
                .filter(|e| e.should_retry())
                .cloned()
                .collect(),
            Err(e) => {
                eprintln!("[sync_queue] Lock poisoned: {}", e);
                vec![]
            }
        }
    }

    /// Marks a sync entry as successfully synced (removes it from queue).
    pub fn mark_synced(&self, task_id: i64) {
        if let Ok(mut store) = self.inner.lock() {
            if store.entries.remove(&task_id).is_some() {
                eprintln!("[sync_queue] Removed synced entry: task_id={}", task_id);
            }
        }
        self.save();
    }

    /// Records a failed sync attempt for an entry.
    pub fn record_failure(&self, task_id: i64, error: String) {
        if let Ok(mut store) = self.inner.lock() {
            if let Some(entry) = store.entries.get_mut(&task_id) {
                entry.record_failure(error.clone());

                eprintln!(
                    "[sync_queue] Recorded failure: task_id={}, attempt={}/{}, error={}, next_retry_in={}s",
                    task_id,
                    entry.retry_count,
                    MAX_RETRY_ATTEMPTS,
                    error,
                    entry.get_backoff_delay_secs()
                );

                // If exhausted, move to failed entries
                if entry.is_exhausted() {
                    if let Some(failed_entry) = store.entries.remove(&task_id) {
                        eprintln!(
                            "[sync_queue] CRITICAL: Entry exhausted max retries, moving to failed: task_id={}, elapsed={}",
                            task_id, failed_entry.elapsed_seconds
                        );
                        store.failed_entries.push(failed_entry);
                    }
                }
            }
        }
        self.save();
    }

    /// Returns true if there are any pending entries.
    pub fn has_pending(&self) -> bool {
        self.inner
            .lock()
            .map(|store| !store.entries.is_empty())
            .unwrap_or(false)
    }

    /// Returns the count of pending entries.
    pub fn pending_count(&self) -> usize {
        self.inner
            .lock()
            .map(|store| store.entries.len())
            .unwrap_or(0)
    }

    /// Returns the count of failed entries (exceeded max retries).
    pub fn failed_count(&self) -> usize {
        self.inner
            .lock()
            .map(|store| store.failed_entries.len())
            .unwrap_or(0)
    }

    /// Clears all pending entries (use with caution!).
    pub fn clear(&self) {
        if let Ok(mut store) = self.inner.lock() {
            store.entries.clear();
        }
        self.save();
    }

    /// Retries all failed entries (moves them back to pending queue).
    pub fn retry_failed(&self) {
        if let Ok(mut store) = self.inner.lock() {
            // Collect entries first to avoid borrow conflict
            let entries_to_retry: Vec<_> = store.failed_entries.drain(..).collect();
            for mut entry in entries_to_retry {
                entry.retry_count = 0;
                entry.last_attempt_at = None;
                entry.last_error = None;
                store.entries.insert(entry.task_id, entry);
            }
            eprintln!("[sync_queue] Moved failed entries back to pending queue");
        }
        self.save();
    }
}

impl Default for SyncQueue {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_backoff_delay() {
        let mut entry = PendingSyncEntry::new(1, 100, "2024-01-01T00:00:00Z".to_string(), None);

        // First retry: 5 seconds
        assert_eq!(entry.get_backoff_delay_secs(), 5);

        entry.retry_count = 1;
        // Second retry: 10 seconds
        assert_eq!(entry.get_backoff_delay_secs(), 10);

        entry.retry_count = 2;
        // Third retry: 20 seconds
        assert_eq!(entry.get_backoff_delay_secs(), 20);

        entry.retry_count = 6;
        // Should cap at MAX_RETRY_DELAY_SECS (300)
        assert_eq!(entry.get_backoff_delay_secs(), 300);
    }

    #[test]
    fn test_exhausted() {
        let mut entry = PendingSyncEntry::new(1, 100, "2024-01-01T00:00:00Z".to_string(), None);

        assert!(!entry.is_exhausted());

        entry.retry_count = MAX_RETRY_ATTEMPTS;
        assert!(entry.is_exhausted());
    }
}
