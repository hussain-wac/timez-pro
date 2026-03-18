use chrono::Utc;
use std::sync::Arc;
use std::time::Duration;
use tauri::AppHandle;

use crate::services::api;
use crate::services::api::AuthToken;
use crate::services::storage::LocalTimeStorage;
use crate::services::timer::TimerState;

pub fn start_idle_monitor(app_handle: AppHandle) {
    std::thread::spawn(move || loop {
        std::thread::sleep(Duration::from_secs(10));

        let timer_state = app_handle.state::<TimerState>();
        let local_store = app_handle.state::<LocalTimeStorage>();
        let auth = app_handle.state::<AuthToken>();

        let token = auth.lock().ok().and_then(|s| s.access_token.clone());
        let is_tracking = timer_state
            .lock()
            .ok()
            .map(|s| s.is_tracking())
            .unwrap_or(false);

        if is_tracking {
            let _ = app_handle.emit("activity-detected", ());
        }
    });
}

pub fn check_and_reset_for_new_day(app_handle: &AppHandle) {
    let timer_state = app_handle.state::<TimerState>();
    let local_store = app_handle.state::<LocalTimeStorage>();

    if let Ok(mut s) = timer_state.lock() {
        let now = Utc::now();

        if now.hour() == 0 && now.minute() == 0 && s.is_tracking() {
            eprintln!("[cron] Midnight reset - stopping timer");
            s.stop_current_local(&local_store);
            let _ = app_handle.emit("midnight-reset", ());
        }
    }
}
