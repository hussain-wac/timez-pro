use std::sync::{Arc, Mutex};

use timez_core::format_duration;
use timez_core::models::{MidnightResetEvent, Project, Task, TimerStatus};
use timez_core::protocol::{Request, ResponseData};
use timez_core::timer_state::TimerStateInner;
use timez_core::api;

use crate::auth_store;
use crate::runtime;
use crate::ServiceKind;

pub fn run(parent_pid: Option<u32>) -> Result<(), String> {
    let timer_state = Arc::new(Mutex::new(TimerStateInner::new()));
    spawn_sync_thread(Arc::clone(&timer_state));

    #[cfg(unix)]
    {
        runtime::run_server(
            ServiceKind::Task.socket_path(),
            parent_pid,
            move |request| handle_request(request, &timer_state),
        )
    }

    #[cfg(windows)]
    {
        runtime::run_server(
            ServiceKind::Task.port(),
            parent_pid,
            move |request| handle_request(request, &timer_state),
        )
    }
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
        Request::CheckMidnightReset => Ok(ResponseData::MidnightReset(check_midnight_reset(
            timer_state,
        )?)),
        Request::ListProjects => Ok(ResponseData::Projects(list_projects()?)),
        Request::ListProjectTasks { project_id } => {
            Ok(ResponseData::Tasks(list_project_tasks(timer_state, project_id)?))
        }
        Request::SetActiveProject { .. } => {
            // Active project is tracked on the frontend, this is a no-op
            Ok(ResponseData::Unit)
        }
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
        (chrono::Utc::now() - started).num_seconds().max(0)
    } else {
        0
    };

    for task in &mut tasks {
        // Check if this task is running locally
        let is_running = running_id == Some(task.id);
        task.running = is_running;

        // Add local elapsed time tracking
        if let Some(base) = timer.base_elapsed.get(&task.id) {
            task.elapsed_secs = *base + if is_running { live_elapsed } else { 0 };
        } else if is_running {
            task.elapsed_secs += live_elapsed;
        }
    }

    Ok(tasks)
}

fn spawn_sync_thread(timer_state: Arc<Mutex<TimerStateInner>>) {
    use timez_core::api;

    const SYNC_INTERVAL_SECS: u64 = 30;

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

            let token = auth_store::read_token();

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
                if let (Some(task_id), Some(started_at)) =
                    (timer.running_task_id, timer.timer_started_at)
                {
                    let total_elapsed = timer.get_total_elapsed(task_id);
                    let client_started = started_at.to_rfc3339();

                    let last_synced = timer.last_synced_elapsed.get(&task_id).copied().unwrap_or(0);
                    let new_time = total_elapsed - last_synced;

                    println!(
                        "[sync] Task {}: total={}, last_synced={}, new={}",
                        task_id, total_elapsed, last_synced, new_time
                    );

                    if total_elapsed > 0 && new_time > 0 {
                        println!("[sync] Syncing {} to server...", format_duration(new_time));

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
                                timer.mark_synced(task_id, total_elapsed);
                                println!("[sync] {} synced successfully", format_duration(new_time));
                            }
                            Err(e) => {
                                println!("[sync] Error: {}", e);
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
