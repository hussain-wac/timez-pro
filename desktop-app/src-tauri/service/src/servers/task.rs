use std::sync::{Arc, Mutex};

use timez_core::api;
use timez_core::format_duration;
use timez_core::models::{MidnightResetEvent, Project, SyncQueueStatus, Task, TimerStatus};
use timez_core::protocol::{Request, ResponseData};
use timez_core::timer_state::TimerStateInner;

use crate::auth_store;
use crate::runtime;
use crate::sync_queue::SyncQueue;
use crate::ServiceKind;

/// Shared state for the task service.
struct TaskServiceState {
    timer_state: Arc<Mutex<TimerStateInner>>,
    sync_queue: Arc<SyncQueue>,
}

pub fn run(parent_pid: Option<u32>) -> Result<(), String> {
    let timer_state = Arc::new(Mutex::new(TimerStateInner::new()));
    let sync_queue = Arc::new(SyncQueue::new());

    spawn_sync_thread(Arc::clone(&timer_state), Arc::clone(&sync_queue));

    let state = Arc::new(TaskServiceState {
        timer_state,
        sync_queue,
    });

    #[cfg(unix)]
    {
        runtime::run_server(
            ServiceKind::Task.socket_path(),
            parent_pid,
            move |request| handle_request(request, &state),
        )
    }

    #[cfg(windows)]
    {
        let state = state.clone();
        runtime::run_server(ServiceKind::Task.port(), parent_pid, move |request| {
            handle_request(request, &state)
        })
    }
}

fn handle_request(
    request: Request,
    state: &Arc<TaskServiceState>,
) -> Result<ResponseData, String> {
    let timer_state = &state.timer_state;
    let sync_queue = &state.sync_queue;

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
        Request::CheckMidnightReset => Ok(ResponseData::MidnightReset(check_midnight_reset(
            timer_state,
        )?)),
        Request::ListProjects => Ok(ResponseData::Projects(list_projects()?)),
        Request::ListProjectTasks { project_id } => Ok(ResponseData::Tasks(list_project_tasks(
            timer_state,
            project_id,
        )?)),
        Request::SetActiveProject { .. } => {
            // Active project is tracked on the frontend, this is a no-op
            Ok(ResponseData::Unit)
        }
        Request::GetSyncStatus => Ok(ResponseData::SyncStatus(get_sync_status(sync_queue))),
        Request::RetrySyncFailed => {
            sync_queue.retry_failed();
            Ok(ResponseData::SyncStatus(get_sync_status(sync_queue)))
        }
        Request::Shutdown => Ok(ResponseData::Unit),
        _ => Err("Unsupported request for task service".to_string()),
    }
}

fn get_sync_status(sync_queue: &Arc<SyncQueue>) -> SyncQueueStatus {
    let pending_count = sync_queue.pending_count();
    let failed_count = sync_queue.failed_count();

    // Get last error from pending entries if any
    let last_error = sync_queue
        .get_entries_to_retry()
        .iter()
        .filter_map(|e| e.last_error.clone())
        .last();

    SyncQueueStatus {
        pending_count,
        failed_count,
        has_errors: failed_count > 0 || pending_count > 0,
        last_error,
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

/// Maximum reasonable elapsed time for a single session (24 hours in seconds).
const MAX_SESSION_SECS: i64 = 24 * 60 * 60;

/// Maximum total elapsed time we'll track (1 year in seconds).
const MAX_TOTAL_ELAPSED_SECS: i64 = 365 * 24 * 60 * 60;

fn get_status(timer_state: &Arc<Mutex<TimerStateInner>>) -> Result<TimerStatus, String> {
    let timer = timer_state.lock().map_err(|err| err.to_string())?;
    Ok(TimerStatus {
        running: timer.running_task_id.is_some(),
        active_task_id: timer.running_task_id,
        current_entry_elapsed: timer
            .timer_started_at
            .map(|started| {
                let elapsed = (chrono::Utc::now() - started).num_seconds();
                // Apply clock protections
                if elapsed < 0 {
                    0
                } else if elapsed > MAX_SESSION_SECS {
                    MAX_SESSION_SECS
                } else {
                    elapsed
                }
            })
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

fn check_midnight_reset(
    timer_state: &Arc<Mutex<TimerStateInner>>,
) -> Result<Option<MidnightResetEvent>, String> {
    let token = auth_store::read_token();
    let mut timer = timer_state.lock().map_err(|err| err.to_string())?;

    match timer.check_midnight_reset(&token) {
        Some(info) => Ok(Some(MidnightResetEvent {
            synced_task_id: info.synced_task_id,
            synced_elapsed: info.synced_elapsed,
            new_date: timer.current_day().to_string(),
        })),
        None => Ok(None),
    }
}

fn list_projects() -> Result<Vec<Project>, String> {
    let token = auth_store::read_token();
    api::list_projects(&token)
}

fn list_project_tasks(
    timer_state: &Arc<Mutex<TimerStateInner>>,
    project_id: i64,
) -> Result<Vec<Task>, String> {
    let token = auth_store::read_token();
    let timer = timer_state.lock().map_err(|err| err.to_string())?;

    // Get tasks from API for this project
    let mut tasks = api::list_project_tasks(project_id, &token)?;

    // Merge with local timer state (running status, live elapsed)
    let running_id = timer.running_task_id;
    let live_elapsed = if let (Some(_), Some(started)) = (running_id, timer.timer_started_at) {
        let elapsed = (chrono::Utc::now() - started).num_seconds();
        // Apply clock protections
        if elapsed < 0 {
            0
        } else if elapsed > MAX_SESSION_SECS {
            MAX_SESSION_SECS
        } else {
            elapsed
        }
    } else {
        0
    };

    for task in &mut tasks {
        // Check if this task is running locally
        let is_running = running_id == Some(task.id);
        task.running = is_running;

        // Add local elapsed time tracking with overflow protection
        if let Some(base) = timer.base_elapsed.get(&task.id) {
            let total = base.saturating_add(if is_running { live_elapsed } else { 0 });
            task.elapsed_secs = total.min(MAX_TOTAL_ELAPSED_SECS);
        } else if is_running {
            let total = task.elapsed_secs.saturating_add(live_elapsed);
            task.elapsed_secs = total.min(MAX_TOTAL_ELAPSED_SECS);
        }
    }

    Ok(tasks)
}

fn spawn_sync_thread(timer_state: Arc<Mutex<TimerStateInner>>, sync_queue: Arc<SyncQueue>) {
    use timez_core::api;

    const SYNC_INTERVAL_SECS: u64 = 30;
    // Shorter interval for checking retry queue
    const RETRY_CHECK_INTERVAL_SECS: u64 = 5;

    std::thread::spawn(move || {
        let mut ticks_since_full_sync: u64 = 0;

        // Initial sync
        {
            let token = auth_store::read_token();
            if let Ok(mut timer) = timer_state.lock() {
                timer.sync_from_api(&token);
                println!("[sync] Initial sync complete");
            }
        }

        loop {
            // Sleep for the shorter retry check interval
            std::thread::sleep(std::time::Duration::from_secs(RETRY_CHECK_INTERVAL_SECS));
            ticks_since_full_sync += RETRY_CHECK_INTERVAL_SECS;

            let token = auth_store::read_token();

            // Process any entries in the retry queue first (with exponential backoff)
            process_retry_queue(&sync_queue, &token);

            // Only do full sync every SYNC_INTERVAL_SECS
            if ticks_since_full_sync < SYNC_INTERVAL_SECS {
                continue;
            }
            ticks_since_full_sync = 0;

            // Check for midnight reset first (before syncing)
            if let Ok(mut timer) = timer_state.lock() {
                if let Some(info) = timer.check_midnight_reset(&token) {
                    println!(
                        "[sync] Midnight reset occurred: synced_task={:?}, synced_elapsed={}",
                        info.synced_task_id, info.synced_elapsed
                    );
                    // After midnight reset, sync from API to get fresh task list
                    timer.sync_from_api(&token);
                    println!("[sync] Post-midnight sync complete");
                    continue;
                }
            }

            println!("[sync] Syncing with API...");

            if let Ok(mut timer) = timer_state.lock() {
                // Use the new slot-based sync method
                if timer.is_running() {
                    match timer.sync_slot(&token) {
                        Some(slot_secs) => {
                            println!("[sync] Slot sync complete: {} seconds", slot_secs);
                            // Also remove from queue if it was there
                            if let Some(task_id) = timer.running_task_id {
                                sync_queue.mark_synced(task_id);
                            }
                        }
                        None => {
                            // Sync failed - add to persistent queue for retry
                            if let (Some(task_id), Some(started_at)) =
                                (timer.running_task_id, timer.timer_started_at)
                            {
                                let total_elapsed = timer.get_total_elapsed(task_id);
                                let client_started = started_at.to_rfc3339();

                                println!("[sync] Slot sync failed, adding to retry queue");
                                sync_queue.enqueue(
                                    task_id,
                                    total_elapsed,
                                    client_started,
                                    None, // Timer still running
                                );

                                eprintln!(
                                    "[sync] SYNC_ERROR: task_id={}, elapsed={}",
                                    task_id, total_elapsed
                                );
                            }
                        }
                    }
                }

                timer.sync_from_api(&token);

                // Log queue status
                let pending = sync_queue.pending_count();
                let failed = sync_queue.failed_count();
                if pending > 0 || failed > 0 {
                    println!(
                        "[sync] Queue status: {} pending, {} failed (exceeded retries)",
                        pending, failed
                    );
                }

                println!("[sync] Sync complete");
            }
        }
    });
}

/// Process entries in the retry queue with exponential backoff.
fn process_retry_queue(sync_queue: &SyncQueue, token: &Option<String>) {
    let entries = sync_queue.get_entries_to_retry();

    if entries.is_empty() {
        return;
    }

    println!(
        "[sync] Processing {} entries from retry queue",
        entries.len()
    );

    for entry in entries {
        println!(
            "[sync] Retrying: task_id={}, elapsed={}, attempt={}",
            entry.task_id,
            entry.elapsed_seconds,
            entry.retry_count + 1
        );

        // For retry queue, we send the stored elapsed as slot_seconds
        let slot_end = entry.client_stopped_at.clone()
            .unwrap_or_else(|| chrono::Utc::now().to_rfc3339());
        let is_final = entry.client_stopped_at.is_some();

        match api::sync_time(
            entry.task_id,
            entry.elapsed_seconds,
            &entry.client_started_at,
            &slot_end,
            is_final,
            token,
        ) {
            Ok(response) => {
                println!(
                    "[sync] Retry successful: task_id={}, backend_duration={:?}",
                    response.task_id, response.duration
                );
                sync_queue.mark_synced(entry.task_id);
            }
            Err(e) => {
                println!(
                    "[sync] Retry failed: task_id={}, error={}",
                    entry.task_id, e
                );
                sync_queue.record_failure(entry.task_id, e);
            }
        }
    }
}
