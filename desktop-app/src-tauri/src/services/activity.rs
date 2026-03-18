use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tauri::AppHandle;

use crate::models::ActivityStats;

pub struct ActivityState {
    pub last_activity: std::time::Instant,
    pub is_idle: bool,
    pub idle_duration_secs: i64,
    pub mouse_movements: Vec<i64>,
    pub keyboard_strokes: Vec<i64>,
}

pub type ActivityStateHandle = Arc<std::sync::Mutex<ActivityState>>;

pub fn new_activity_state() -> ActivityStateHandle {
    Arc::new(std::sync::Mutex::new(ActivityState {
        last_activity: std::time::Instant::now(),
        is_idle: false,
        idle_duration_secs: 0,
        mouse_movements: vec![],
        keyboard_strokes: vec![],
    }))
}

pub fn spawn_idle_monitor(app_handle: AppHandle, idle_threshold_secs: u64) {
    std::thread::spawn(move || loop {
        std::thread::sleep(Duration::from_secs(5));

        let activity_state = app_handle.state::<ActivityStateHandle>();

        if let Ok(mut state) = activity_state.lock() {
            let now = std::time::Instant::now();
            let idle_time = now.duration_since(state.last_activity).as_secs();

            if idle_time >= idle_threshold_secs && !state.is_idle {
                state.is_idle = true;
                state.idle_duration_secs = idle_time as i64;
                let _ = app_handle.emit(
                    "idle-detected",
                    serde_json::json!({
                        "idle_duration": state.idle_duration_secs
                    }),
                );
                eprintln!("[idle] Idle detected: {} seconds", state.idle_duration_secs);
            } else if idle_time < idle_threshold_secs && state.is_idle {
                state.is_idle = false;
                state.idle_duration_secs = 0;
            }
        }
    });
}

pub fn reset_activity(activity_state: &ActivityStateHandle) {
    if let Ok(mut state) = activity_state.lock() {
        state.last_activity = std::time::Instant::now();
        state.is_idle = false;
        state.idle_duration_secs = 0;
    }
}

pub fn record_mouse_movement(activity_state: &ActivityStateHandle) {
    if let Ok(mut state) = activity_state.lock() {
        state.mouse_movements.push(chrono::Utc::now().timestamp());
        state.last_activity = std::time::Instant::now();
    }
}

pub fn record_keyboard_stroke(activity_state: &ActivityStateHandle) {
    if let Ok(mut state) = activity_state.lock() {
        state.keyboard_strokes.push(chrono::Utc::now().timestamp());
        state.last_activity = std::time::Instant::now();
    }
}

pub fn get_activity_stats(activity: ActivityStateHandle) -> Result<ActivityStats, String> {
    let state = activity.lock().map_err(|e| e.to_string())?;

    Ok(ActivityStats {
        is_idle: state.is_idle,
        idle_duration_secs: state.idle_duration_secs,
        mouse_movements_count: state.mouse_movements.len() as i64,
        keyboard_strokes_count: state.keyboard_strokes.len() as i64,
    })
}
