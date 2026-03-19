use serde::{Deserialize, Serialize};

use crate::models::{ActivityStats, AuthResponse, AuthUser, IdleEvent, MidnightResetEvent, Project, Task, TimerStatus};

#[derive(Debug, Serialize, Deserialize)]
pub struct RequestEnvelope {
    pub token: String,
    pub request: Request,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum Request {
    ListTasks,
    StartTimer { task_id: i64 },
    StopTimer,
    GetStatus,
    AddIdleTime { task_id: i64, duration_secs: i64 },
    DiscardIdleTime { task_id: i64 },
    RefreshTasks,
    GetActivityStats,
    GoogleLogin { google_id_token: String },
    StartGoogleAuth { client_id: String, client_secret: String },
    ValidateToken { token: String },
    Logout,
    GetIdleEvent,
    ResolveIdleEvent,
    /// Check if midnight has passed and get reset event if it occurred
    CheckMidnightReset,
    /// List all projects the user is allocated to
    ListProjects,
    /// List tasks for a specific project
    ListProjectTasks { project_id: i64 },
    /// Set the currently active project for filtering
    SetActiveProject { project_id: Option<i64> },
    Shutdown,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ResponseEnvelope {
    pub ok: bool,
    pub data: Option<ResponseData>,
    pub error: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum ResponseData {
    Tasks(Vec<Task>),
    Status(TimerStatus),
    Activity(ActivityStats),
    AuthResponse(AuthResponse),
    AuthUser(AuthUser),
    IdleEvent(Option<IdleEvent>),
    /// Midnight reset event (None if no reset occurred)
    MidnightReset(Option<MidnightResetEvent>),
    /// List of projects
    Projects(Vec<Project>),
    Unit,
}
