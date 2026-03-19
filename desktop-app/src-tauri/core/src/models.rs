use serde::{Deserialize, Serialize};

/// A project that contains tasks.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub id: i64,
    pub name: String,
    pub color: Option<String>,
    pub task_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: i64,
    pub name: String,
    pub budget_secs: i64,
    pub elapsed_secs: i64,
    pub running: bool,
    /// Project this task belongs to
    pub project_id: Option<i64>,
    /// Project name (for display)
    pub project_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimerStatus {
    pub running: bool,
    pub active_task_id: Option<i64>,
    pub current_entry_elapsed: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct IdleEvent {
    pub idle_duration_secs: i64,
    pub task_id: i64,
    pub task_name: String,
    pub tracking_active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivityStats {
    pub active_secs: i64,
    pub idle_secs: i64,
    pub total_secs: i64,
    pub activity_percent: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthUser {
    pub id: i64,
    pub email: String,
    pub name: Option<String>,
    pub picture: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthResponse {
    pub access_token: String,
    pub user: AuthUser,
}

/// Event emitted when midnight timer reset occurs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MidnightResetEvent {
    /// Task ID that was running and synced before reset (if any)
    pub synced_task_id: Option<i64>,
    /// Elapsed seconds that were synced before reset
    pub synced_elapsed: i64,
    /// The new date (today) after the reset
    pub new_date: String,
}

/// Status of the sync queue showing pending and failed entries.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncQueueStatus {
    /// Number of entries pending sync (will be retried)
    pub pending_count: usize,
    /// Number of entries that have exceeded max retries
    pub failed_count: usize,
    /// Whether there are any entries that need user attention
    pub has_errors: bool,
    /// Last sync error message (if any)
    pub last_error: Option<String>,
}
