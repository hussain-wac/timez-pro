use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
use std::thread;
use std::time::Duration;

use chrono::Utc;
use tauri::AppHandle;
use tauri_plugin_notification::NotificationExt;

use crate::api;
use crate::api::AuthToken;
use crate::local_store::LocalTimeStorage;
use crate::timer_state::TimerState;

/// Send a desktop notification
fn send_notification(app_handle: &AppHandle, title: &str, body: &str) {
    if let Err(e) = app_handle
        .notification()
        .builder()
        .title(title)
        .body(body)
        .show()
    {
        eprintln!("[notification] Failed to send: {}", e);
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

/// Data captured for sync before releasing locks
#[derive(Clone)]
struct SyncTask {
    task_id: i64,
    total_elapsed: i64,
    client_started: String,
    client_stopped: String,
}

pub fn quit_app(app_handle: AppHandle) -> Result<(), String> {
    eprintln!("[quit] Starting graceful shutdown...");

    // Get token early
    let token = {
        let auth = app_handle.state::<AuthToken>();
        auth.inner()
            .lock()
            .ok()
            .and_then(|s| s.access_token.clone())
    };

    let local_store = app_handle.state::<LocalTimeStorage>();
    let timer_state = app_handle.state::<TimerState>();

    // Capture running task data BEFORE releasing lock
    let running_sync_task: Option<SyncTask> = {
        match timer_state.inner().lock() {
            Ok(s) => {
                if let Some(task_id) = s.running_task_id {
                    let live_elapsed = s
                        .timer_started_at
                        .map(|started| (Utc::now() - started).num_seconds().max(0))
                        .unwrap_or(0);
                    let base_elapsed = s.base_elapsed.get(&task_id).copied().unwrap_or(0);
                    let total_elapsed = base_elapsed + live_elapsed;
                    let client_started = s.client_started_at.clone()
                        .unwrap_or_else(|| Utc::now().to_rfc3339());
                    let client_stopped = Utc::now().to_rfc3339();

                    eprintln!(
                        "[quit] Running task {}: base={}, live={}, total={}",
                        task_id, base_elapsed, live_elapsed, total_elapsed
                    );

                    Some(SyncTask {
                        task_id,
                        total_elapsed,
                        client_started,
                        client_stopped,
                    })
                } else {
                    None
                }
            }
            Err(e) => {
                eprintln!("[quit] CRITICAL: Timer state mutex poisoned: {}", e);
                // Try to recover
                timer_state.clear_poison();
                None
            }
        }
    };

    // Stop the timer locally (quick operation)
    if let Ok(mut s) = timer_state.inner().lock() {
        if s.running_task_id.is_some() {
            s.stop_current_local(&local_store);
        }
    }

    // Collect all pending entries for sync
    let pending_entries: Vec<_> = local_store.get_unsynced_entries()
        .into_iter()
        .filter(|e| {
            let total = e.timestamps.last().map(|t| t.elapsed_secs).unwrap_or(0);
            !e.synced && total > e.last_synced_elapsed
        })
        .collect();

    let total_sync_count = running_sync_task.is_some() as usize + pending_entries.len();

    if total_sync_count == 0 {
        eprintln!("[quit] No pending syncs, exiting immediately");
        app_handle.exit(0);
        return Ok(());
    }

    // Show notification
    send_notification(
        &app_handle,
        "Timez Pro - Saving",
        &format!("Saving {} pending entries...", total_sync_count)
    );

    // Clone what we need for the sync thread
    let app_handle_clone = app_handle.clone();
    let token_clone = token.clone();
    let local_store_clone = local_store.inner().clone();
    let sync_complete = Arc::new(AtomicBool::new(false));
    let sync_complete_clone = Arc::clone(&sync_complete);

    // Spawn sync thread to avoid blocking UI
    thread::spawn(move || {
        let mut synced_count = 0;
        let mut failed_count = 0;

        // Sync running task first
        if let Some(task) = running_sync_task {
            if task.total_elapsed > 0 {
                eprintln!(
                    "[quit] Syncing running task {} with {} seconds...",
                    task.task_id, task.total_elapsed
                );

                // Retry up to 3 times
                let mut success = false;
                for attempt in 1..=3 {
                    match api::sync_time(
                        task.task_id,
                        task.total_elapsed,
                        &task.client_started,
                        Some(&task.client_stopped),
                        &token_clone,
                    ) {
                        Ok(response) => {
                            eprintln!(
                                "[quit] Handshake confirmed (attempt {}): task_id={}, duration={:?}",
                                attempt, response.task_id, response.duration
                            );
                            local_store_clone.mark_synced(task.task_id, task.total_elapsed);
                            synced_count += 1;
                            success = true;
                            break;
                        }
                        Err(e) => {
                            eprintln!("[quit] Sync attempt {} failed: {}", attempt, e);
                            if attempt < 3 {
                                thread::sleep(Duration::from_millis(300));
                            }
                        }
                    }
                }

                if !success {
                    failed_count += 1;
                }
            }
        }

        // Sync other pending entries
        for entry in pending_entries {
            let task_id = entry.task_id;
            let total_elapsed: i64 = entry.timestamps.last().map(|t| t.elapsed_secs).unwrap_or(0);

            if total_elapsed > entry.last_synced_elapsed {
                match api::sync_time(
                    task_id,
                    total_elapsed,
                    &entry.client_started_at,
                    entry.client_stopped_at.as_deref(),
                    &token_clone,
                ) {
                    Ok(response) => {
                        eprintln!(
                            "[quit] Synced entry: task_id={}, duration={:?}",
                            response.task_id, response.duration
                        );
                        local_store_clone.mark_synced(task_id, total_elapsed);
                        synced_count += 1;
                    }
                    Err(e) => {
                        eprintln!("[quit] Failed to sync task {}: {}", task_id, e);
                        failed_count += 1;
                    }
                }
            }
        }

        // Clean up
        local_store_clone.cleanup_completed_entries();

        eprintln!(
            "[quit] Sync complete: {} synced, {} failed",
            synced_count, failed_count
        );

        // Send final notification
        if failed_count == 0 {
            send_notification(
                &app_handle_clone,
                "Timez Pro - Saved",
                &format!("All {} entries saved successfully!", synced_count)
            );
        } else {
            send_notification(
                &app_handle_clone,
                "Timez Pro - Warning",
                &format!("{} saved, {} failed", synced_count, failed_count)
            );
        }

        sync_complete_clone.store(true, Ordering::SeqCst);

        // Small delay for notification to show
        thread::sleep(Duration::from_millis(500));

        // Exit the app
        app_handle_clone.exit(0);
    });

    // Return immediately - the sync thread will exit the app when done
    // But wait a short time to ensure the thread starts
    thread::sleep(Duration::from_millis(50));

    Ok(())
}
