use tauri::State;

use crate::models::Task;
use crate::timer_state::TimerState;

pub fn list_tasks(timer: State<'_, TimerState>) -> Result<Vec<Task>, String> {
    let s = timer.lock().map_err(|e| e.to_string())?;
    Ok(s.get_tasks())
}
