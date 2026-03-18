use std::sync::{Arc, Mutex};

use chrono::Timelike;
use notify_rust::Notification;
use timez_core::models::{Task, TimerStatus};
use timez_core::protocol::{Request, ResponseData};
use timez_core::timer_state::TimerStateInner;

use crate::auth_store;
use crate::runtime;
use crate::ServiceKind;

/// Send a desktop notification
fn send_notification(title: &str, body: &str) {
    println!("[notification] Attempting to send: {} - {}", title, body);

    match Notification::new()
        .summary(title)
        .body(body)
        .appname("Timez Pro")
        .timeout(notify_rust::Timeout::Milliseconds(4000))
        .show()
    {
        Ok(handle) => {
            println!("[notification] Sent successfully: {:?}", handle);
        }
        Err(e) => {
            eprintln!("[notification] Failed to send: {}", e);
            // Try fallback using notify-send command
            let _ = std::process::Command::new("notify-send")
                .arg("-a")
                .arg("Timez Pro")
                .arg("-t")
                .arg("4000")
                .arg(title)
                .arg(body)
                .spawn();
        }
    }
}

/// Format seconds into a human-readable string
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

pub fn run(parent_pid: Option<u32>) -> Result<(), String> {
    let timer_state = Arc::new(Mutex::new(TimerStateInner::new()));
    spawn_sync_thread(Arc::clone(&timer_state));

    runtime::run_server(
        ServiceKind::Task.socket_path(),
        parent_pid,
        move |request| handle_request(request, &timer_state),
    )
}

fn handle_request(
    request: Request,
    timer_state: &Arc<Mutex<TimerStateInner>>,
) -> Result<ResponseData, String> {
    match request {
        Request::ListTasks => Ok(ResponseData::Tasks(list_tasks(timer_state)?)),
        Request::StartTimer { task_id } => {
            Ok(ResponseData::Tasks(start_timer(timer_state, task_id)?))
        }
        Request::StopTimer => Ok(ResponseData::Tasks(stop_timer(timer_state)?)),
        Request::GetStatus => Ok(ResponseData::Status(get_status(timer_state)?)),
        Request::AddIdleTime {
            task_id,
            duration_secs,
        } => Ok(ResponseData::Tasks(add_idle_time(
            timer_state,
            task_id,
            duration_secs,
        )?)),
        Request::DiscardIdleTime { task_id } => Ok(ResponseData::Tasks(discard_idle_time(
            timer_state,
            task_id,
        )?)),
        Request::RefreshTasks => Ok(ResponseData::Tasks(refresh_tasks(timer_state)?)),
        Request::Shutdown => Ok(ResponseData::Unit),
        _ => Err("Unsupported request for task service".to_string()),
    }
}

fn list_tasks(timer_state: &Arc<Mutex<TimerStateInner>>) -> Result<Vec<Task>, String> {
    let timer = timer_state.lock().map_err(|err| err.to_string())?;
    Ok(timer.get_tasks())
}

fn start_timer(
    timer_state: &Arc<Mutex<TimerStateInner>>,
    task_id: i64,
) -> Result<Vec<Task>, String> {
    let token = auth_store::read_token();
    let mut timer = timer_state.lock().map_err(|err| err.to_string())?;
    timer.start_task(task_id, &token)?;
    Ok(timer.get_tasks())
}

fn stop_timer(timer_state: &Arc<Mutex<TimerStateInner>>) -> Result<Vec<Task>, String> {
    let token = auth_store::read_token();
    let mut timer = timer_state.lock().map_err(|err| err.to_string())?;
    timer.stop_current(&token)?;
    Ok(timer.get_tasks())
}

fn get_status(timer_state: &Arc<Mutex<TimerStateInner>>) -> Result<TimerStatus, String> {
    let timer = timer_state.lock().map_err(|err| err.to_string())?;
    Ok(TimerStatus {
        running: timer.running_task_id.is_some(),
        active_task_id: timer.running_task_id,
        current_entry_elapsed: timer
            .timer_started_at
            .map(|started| (chrono::Utc::now() - started).num_seconds().max(0))
            .unwrap_or(0),
    })
}

fn add_idle_time(
    timer_state: &Arc<Mutex<TimerStateInner>>,
    task_id: i64,
    duration_secs: i64,
) -> Result<Vec<Task>, String> {
    let token = auth_store::read_token();
    let mut timer = timer_state.lock().map_err(|err| err.to_string())?;
    timer.resume_with_idle_time(task_id, duration_secs, &token)?;
    Ok(timer.get_tasks())
}

fn discard_idle_time(
    timer_state: &Arc<Mutex<TimerStateInner>>,
    task_id: i64,
) -> Result<Vec<Task>, String> {
    let token = auth_store::read_token();
    let mut timer = timer_state.lock().map_err(|err| err.to_string())?;
    timer.start_task(task_id, &token)?;
    Ok(timer.get_tasks())
}

fn refresh_tasks(timer_state: &Arc<Mutex<TimerStateInner>>) -> Result<Vec<Task>, String> {
    let token = auth_store::read_token();
    let mut timer = timer_state.lock().map_err(|err| err.to_string())?;
    timer.sync_from_api(&token);
    Ok(timer.get_tasks())
}

fn spawn_sync_thread(timer_state: Arc<Mutex<TimerStateInner>>) {
    use timez_core::api;

    const SYNC_INTERVAL_SECS: u64 = 30; // Sync every 30 seconds

    std::thread::spawn(move || {
        // Initial sync
        {
            let token = auth_store::read_token();
            if let Ok(mut timer) = timer_state.lock() {
                timer.sync_from_api(&token);
                println!("[sync] Initial sync complete");
            }
        }

        loop {
            std::thread::sleep(std::time::Duration::from_secs(SYNC_INTERVAL_SECS));

            let now = chrono::Utc::now();

            // Check for midnight reset
            if now.hour() == 0 && now.minute() == 0 {
                println!("[sync] Midnight reset - stopping timer");
                send_notification("Timez Pro", "Midnight reset - timer stopped");
                if let Ok(mut timer) = timer_state.lock() {
                    if let Some(task_id) = timer.running_task_id {
                        // Get total elapsed using the new method
                        let total_elapsed = timer.get_total_elapsed(task_id);
                        if total_elapsed > 0 {
                            if let Some(started_at) = timer.timer_started_at {
                                let client_started = started_at.to_rfc3339();
                                let _ = api::sync_time(
                                    task_id,
                                    total_elapsed,
                                    &client_started,
                                    Some(&now.to_rfc3339()),
                                    &auth_store::read_token(),
                                );
                            }
                        }
                        let _ = timer.stop_current(&auth_store::read_token());
                    }
                }
            }

            let token = auth_store::read_token();
            println!("[sync] Syncing with API...");

            if let Ok(mut timer) = timer_state.lock() {
                // Get running task info
                if let (Some(task_id), Some(started_at)) =
                    (timer.running_task_id, timer.timer_started_at)
                {
                    // Calculate TOTAL elapsed (base + live) - this is cumulative
                    let total_elapsed = timer.get_total_elapsed(task_id);
                    let client_started = started_at.to_rfc3339();

                    // Only sync if we have new time to sync
                    let last_synced = timer.last_synced_elapsed.get(&task_id).copied().unwrap_or(0);
                    let new_time = total_elapsed - last_synced;

                    println!(
                        "[sync] Task {}: total={}, last_synced={}, new={}",
                        task_id, total_elapsed, last_synced, new_time
                    );

                    if total_elapsed > 0 && new_time > 0 {
                        // Show notification before syncing
                        send_notification(
                            "Timez Pro - Syncing",
                            &format!("Syncing {} to server...", format_duration(new_time))
                        );

                        // Send TOTAL elapsed (cumulative) to backend
                        match api::sync_time(task_id, total_elapsed, &client_started, None, &token) {
                            Ok(response) => {
                                println!(
                                    "[sync] Handshake confirmed: task_id={}, backend_duration={:?}",
                                    response.task_id, response.duration
                                );
                                println!(
                                    "[sync] Synced {} total ({} new) for task {}",
                                    total_elapsed, new_time, task_id
                                );

                                // Mark as synced - this resets timer_started_at
                                timer.mark_synced(task_id, total_elapsed);

                                // Success notification
                                send_notification(
                                    "Timez Pro - Synced",
                                    &format!("{} synced successfully", format_duration(new_time))
                                );
                            }
                            Err(e) => {
                                println!("[sync] Error: {}", e);
                                send_notification(
                                    "Timez Pro - Sync Failed",
                                    &format!("Failed to sync: {}", e)
                                );
                            }
                        }
                    }
                }

                timer.sync_from_api(&token);
                println!("[sync] Sync complete");
            }
        }
    });
}
