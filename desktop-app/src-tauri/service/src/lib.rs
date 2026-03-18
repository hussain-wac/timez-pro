pub mod auth_store;
pub mod idle_detection;
pub mod runtime;
pub mod servers;

#[cfg(unix)]
use std::path::PathBuf;

pub const REQUEST_TOKEN: &str = "timez-local";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ServiceKind {
    Auth,
    Task,
    Tracker,
    IdleTime,
    Quit,
}

impl ServiceKind {
    #[cfg(unix)]
    pub fn socket_path(self) -> PathBuf {
        PathBuf::from(match self {
            ServiceKind::Auth => "/tmp/timez-auth-service.sock",
            ServiceKind::Task => "/tmp/timez-task-service.sock",
            ServiceKind::Tracker => "/tmp/timez-tracker-service.sock",
            ServiceKind::IdleTime => "/tmp/timez-idle-time-service.sock",
            ServiceKind::Quit => "/tmp/timez-quit-service.sock",
        })
    }

    #[cfg(windows)]
    pub fn port(self) -> u16 {
        match self {
            ServiceKind::Auth => 23401,
            ServiceKind::Task => 23402,
            ServiceKind::Tracker => 23403,
            ServiceKind::IdleTime => 23404,
            ServiceKind::Quit => 23405,
        }
    }

    pub fn binary_name(self) -> &'static str {
        match self {
            ServiceKind::Auth => "timez-auth-service",
            ServiceKind::Task => "timez-task-service",
            ServiceKind::Tracker => "timez-tracker-service",
            ServiceKind::IdleTime => "timez-idle-time-service",
            ServiceKind::Quit => "timez-quit-service",
        }
    }
}
