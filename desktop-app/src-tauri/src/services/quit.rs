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

pub fn quit_app(app_handle: AppHandle) -> Result<(), String> {
    let token = {
        let auth = app_handle.state::<AuthToken>();
        auth.inner()
            .lock()
            .ok()
            .and_then(|s| s.access_token.clone())
    };

    let local_store = app_handle.state::<LocalTimeStorage>();
    let timer_state = app_handle.state::<TimerState>();

    // Stop the current timer locally first
    if let Ok(mut s) = timer_state.inner().lock() {
        if s.running_task_id.is_some() {
            s.stop_current_local(&local_store);
        }
    }

    // Sync all unsynced entries with handshake confirmation
    let entries = local_store.get_unsynced_entries();
    let total_entries = entries.len();
    let mut synced_count = 0;
    let mut failed_count = 0;

    if total_entries > 0 {
        send_notification(
            &app_handle,
            "Timez Pro - Closing",
            &format!("Syncing {} pending entries before exit...", total_entries)
        );
    }

    for entry in entries {
        if entry.synced {
            continue;
        }

        let task_id = entry.task_id;
        let client_started_at = entry.client_started_at.clone();
        let client_stopped_at = entry.client_stopped_at.clone();
        // Get elapsed from the LAST timestamp (cumulative, not delta)
        let total_elapsed: i64 = entry.timestamps.last().map(|t| t.elapsed_secs).unwrap_or(0);

        if total_elapsed > 0 {
            eprintln!(
                "[quit] Syncing task {} with {} seconds...",
                task_id, total_elapsed
            );

            match api::sync_time(
                task_id,
                total_elapsed,
                &client_started_at,
                client_stopped_at.as_deref(),
                &token,
            ) {
                Ok(response) => {
                    // Handshake successful - backend confirmed receipt
                    eprintln!(
                        "[quit] Handshake confirmed: task_id={}, backend_duration={:?}, is_synced={}",
                        response.task_id, response.duration, response.is_synced
                    );
                    local_store.mark_synced(task_id, total_elapsed);
                    synced_count += 1;
                    eprintln!(
                        "[quit] Successfully synced {} for task {}",
                        format_duration(total_elapsed), task_id
                    );
                }
                Err(e) => {
                    eprintln!("[quit] Failed to sync task {}: {}", task_id, e);
                    failed_count += 1;
                    send_notification(
                        &app_handle,
                        "Timez Pro - Sync Error",
                        &format!("Failed to sync task {}: {}", task_id, e)
                    );
                }
            }
        }
    }

    // Final notification based on sync results
    if synced_count > 0 && failed_count == 0 {
        send_notification(
            &app_handle,
            "Timez Pro - Goodbye",
            &format!("All {} entries synced successfully!", synced_count)
        );
    } else if failed_count > 0 {
        send_notification(
            &app_handle,
            "Timez Pro - Warning",
            &format!("{} entries synced, {} failed. Data may be lost.", synced_count, failed_count)
        );
    }

    eprintln!("[quit] Sync complete ({} synced, {} failed), exiting app", synced_count, failed_count);

    // Small delay to ensure notifications are shown
    std::thread::sleep(std::time::Duration::from_millis(500));

    app_handle.exit(0);
    Ok(())
}
