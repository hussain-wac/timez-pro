use std::sync::{Arc, Mutex};
use std::time::Duration;

use chrono::Utc;
use timez_core::constants::{ACTIVE_THRESHOLD_SECS, DEFAULT_IDLE_THRESHOLD_SECS, POLL_INTERVAL_SECS};
use timez_core::models::{IdleEvent, Task};
use timez_core::protocol::{Request, ResponseData};

use crate::idle_detection::IdleDetector;
use crate::runtime;
use crate::ServiceKind;

pub fn run(parent_pid: Option<u32>) -> Result<(), String> {
    let pending_idle_event = Arc::new(Mutex::new(None));
    spawn_idle_monitor(Arc::clone(&pending_idle_event));

    #[cfg(unix)]
    {
        runtime::run_server(
            ServiceKind::IdleTime.socket_path(),
            parent_pid,
            move |request| match request {
                Request::GetIdleEvent => {
                    let pending = pending_idle_event.lock().map_err(|err| err.to_string())?;
                    Ok(ResponseData::IdleEvent(pending.clone()))
                }
                Request::ResolveIdleEvent => {
                    let mut pending = pending_idle_event.lock().map_err(|err| err.to_string())?;
                    *pending = None;
                    Ok(ResponseData::Unit)
                }
                Request::Shutdown => Ok(ResponseData::Unit),
                _ => Err("Unsupported request for idle-time service".to_string()),
            },
        )
    }

    #[cfg(windows)]
    {
        runtime::run_server(
            ServiceKind::IdleTime.port(),
            parent_pid,
            move |request| match request {
                Request::GetIdleEvent => {
                    let pending = pending_idle_event.lock().map_err(|err| err.to_string())?;
                    Ok(ResponseData::IdleEvent(pending.clone()))
                }
                Request::ResolveIdleEvent => {
                    let mut pending = pending_idle_event.lock().map_err(|err| err.to_string())?;
                    *pending = None;
                    Ok(ResponseData::Unit)
                }
                Request::Shutdown => Ok(ResponseData::Unit),
                _ => Err("Unsupported request for idle-time service".to_string()),
            },
        )
    }
}

fn spawn_idle_monitor(pending_idle_event: Arc<Mutex<Option<IdleEvent>>>) {
    std::thread::spawn(move || {
        let detector = match IdleDetector::new() {
            Ok(d) => d,
            Err(err) => {
                eprintln!("[idle-time] Failed to initialize idle detector: {err}");
                eprintln!("[idle-time] Idle detection will be disabled");
                return;
            }
        };

        eprintln!(
            "[idle-time] Idle monitor started (threshold={}s, poll={}s)",
            DEFAULT_IDLE_THRESHOLD_SECS,
            POLL_INTERVAL_SECS
        );

        let mut is_idle = false;
        let mut idle_started_at: Option<chrono::DateTime<Utc>> = None;
        let mut paused_task: Option<Task> = None;
        let mut was_locked = false;

        loop {
            std::thread::sleep(Duration::from_secs(POLL_INTERVAL_SECS));

            let is_locked = detector.is_locked();
            let system_idle_secs = detector.get_idle_secs();

            let user_is_active = system_idle_secs < ACTIVE_THRESHOLD_SECS && !is_locked;

            // Log lock state changes
            if is_locked && !was_locked {
                was_locked = true;
                eprintln!("[idle-time] Session locked - treating as idle");
            } else if !is_locked && was_locked {
                was_locked = false;
                eprintln!("[idle-time] Session unlocked");
            }

            // User became active again
            if user_is_active {
                if is_idle {
                    // User was idle but is now active - create idle event for UI
                    if let (Some(task), Some(started_at)) = (paused_task.take(), idle_started_at) {
                        let idle_duration_secs = (Utc::now() - started_at).num_seconds().max(0);
                        eprintln!(
                            "[idle-time] User returned after {}s idle, task: {}",
                            idle_duration_secs, task.name
                        );
                        if let Ok(mut pending) = pending_idle_event.lock() {
                            *pending = Some(IdleEvent {
                                idle_duration_secs,
                                task_id: task.id,
                                task_name: task.name,
                                tracking_active: false,
                            });
                        }
                    }

                    is_idle = false;
                    idle_started_at = None;
                }
                continue;
            }

            // User became idle (or session locked)
            if !is_idle && (system_idle_secs >= DEFAULT_IDLE_THRESHOLD_SECS || is_locked) {
                let running_task = current_running_task();
                if let Some(task) = running_task {
                    eprintln!(
                        "[idle-time] User idle for {}s (locked={}), stopping task: {}",
                        system_idle_secs, is_locked, task.name
                    );

                    // Stop the timer
                    send_stop_timer();

                    paused_task = Some(task);
                    idle_started_at =
                        Some(Utc::now() - chrono::Duration::seconds(system_idle_secs as i64));
                    is_idle = true;

                    // Update pending event
                    if let (Some(task), Some(started_at)) = (&paused_task, idle_started_at) {
                        if let Ok(mut pending) = pending_idle_event.lock() {
                            *pending = Some(IdleEvent {
                                idle_duration_secs: (Utc::now() - started_at).num_seconds().max(0),
                                task_id: task.id,
                                task_name: task.name.clone(),
                                tracking_active: true,
                            });
                        }
                    }
                }
            }

            // Continue updating idle duration while idle
            if is_idle {
                if let (Some(task), Some(started_at)) = (&paused_task, idle_started_at) {
                    if let Ok(mut pending) = pending_idle_event.lock() {
                        *pending = Some(IdleEvent {
                            idle_duration_secs: (Utc::now() - started_at).num_seconds().max(0),
                            task_id: task.id,
                            task_name: task.name.clone(),
                            tracking_active: true,
                        });
                    }
                }
            }
        }
    });
}

fn current_running_task() -> Option<Task> {
    #[cfg(unix)]
    {
        let response =
            runtime::send_request(&ServiceKind::Task.socket_path(), Request::ListTasks).ok()?;
        match response {
            ResponseData::Tasks(tasks) => tasks.into_iter().find(|task| task.running),
            _ => None,
        }
    }

    #[cfg(windows)]
    {
        let response =
            runtime::send_request(ServiceKind::Task.port(), Request::ListTasks).ok()?;
        match response {
            ResponseData::Tasks(tasks) => tasks.into_iter().find(|task| task.running),
            _ => None,
        }
    }
}

fn send_stop_timer() {
    #[cfg(unix)]
    {
        let _ = runtime::send_request(&ServiceKind::Task.socket_path(), Request::StopTimer);
    }

    #[cfg(windows)]
    {
        let _ = runtime::send_request(ServiceKind::Task.port(), Request::StopTimer);
    }
}
