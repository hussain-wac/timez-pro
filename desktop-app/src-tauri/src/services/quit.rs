use tauri::AppHandle;

use crate::api;
use crate::api::AuthToken;
use crate::local_store::LocalTimeStorage;
use crate::timer_state::TimerState;

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

    // Sync all unsynced entries
    let entries = local_store.get_unsynced_entries();
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
            let _ = api::sync_time(
                task_id,
                total_elapsed,
                &client_started_at,
                client_stopped_at.as_deref(),
                &token,
            );
            local_store.mark_synced(task_id, total_elapsed);
            eprintln!(
                "[quit] Synced {} seconds for task {}",
                total_elapsed, task_id
            );
        }
    }

    eprintln!("[quit] Sync complete, exiting app");
    app_handle.exit(0);
    Ok(())
}
