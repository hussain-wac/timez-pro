//! Cross-platform idle detection for service mode.
//!
//! Monitors user activity:
//! - Linux: D-Bus (Mutter IdleMonitor, freedesktop.ScreenSaver)
//! - macOS: ioreg command for HIDIdleTime
//! - Windows: Win32 GetLastInputInfo API

use std::sync::{Arc, Mutex as StdMutex};
use std::time::Duration;

use chrono::Utc;

use crate::api::AuthTokenState;
use crate::constants::POLL_INTERVAL_SECS;
use crate::models::{ActivityStats, IdleEvent};
use crate::timer_state::TimerStateInner;

/// Tracks user activity statistics over time.
#[derive(Debug)]
pub struct ActivityTracker {
    pub active_secs: i64,
    pub idle_secs: i64,
    pub last_activity_at: chrono::DateTime<Utc>,
}

impl Default for ActivityTracker {
    fn default() -> Self {
        Self::new()
    }
}

impl ActivityTracker {
    #[must_use]
    pub fn new() -> Self {
        Self {
            active_secs: 0,
            idle_secs: 0,
            last_activity_at: Utc::now(),
        }
    }

    #[inline]
    pub fn record_active(&mut self, secs: i64) {
        self.active_secs += secs;
        self.last_activity_at = Utc::now();
    }

    #[inline]
    pub fn record_idle(&mut self, secs: i64) {
        self.idle_secs += secs;
    }

    #[must_use]
    pub fn stats(&self) -> ActivityStats {
        let total = self.active_secs + self.idle_secs;
        let percent = if total > 0 {
            (self.active_secs as f64 / total as f64) * 100.0
        } else {
            100.0
        };
        ActivityStats {
            active_secs: self.active_secs,
            idle_secs: self.idle_secs,
            total_secs: total,
            activity_percent: (percent * 10.0).round() / 10.0,
        }
    }

    pub fn reset(&mut self) {
        self.active_secs = 0;
        self.idle_secs = 0;
        self.last_activity_at = Utc::now();
    }
}

pub type ActivityState = StdMutex<ActivityTracker>;

// ============================================================================
// Platform-specific idle detection
// ============================================================================

#[cfg(target_os = "linux")]
mod platform {
    use dbus::blocking::Connection;
    use std::time::Duration;

    pub struct IdleDetector {
        conn: Connection,
    }

    impl IdleDetector {
        pub fn new() -> Result<Self, String> {
            let conn =
                Connection::new_session().map_err(|e| format!("D-Bus connection failed: {}", e))?;
            Ok(Self { conn })
        }

        pub fn get_idle_secs(&self) -> u64 {
            self.query_mutter_idle()
                .or_else(|| self.query_freedesktop_idle())
                .unwrap_or(0)
        }

        pub fn is_locked(&self) -> bool {
            self.check_gnome_screensaver()
                .or_else(|| self.check_freedesktop_screensaver())
                .or_else(|| self.check_logind_locked())
                .unwrap_or(false)
        }

        fn query_mutter_idle(&self) -> Option<u64> {
            let proxy = self.conn.with_proxy(
                "org.gnome.Mutter.IdleMonitor",
                "/org/gnome/Mutter/IdleMonitor/Core",
                Duration::from_millis(500),
            );
            let result: Result<(u64,), _> =
                proxy.method_call("org.gnome.Mutter.IdleMonitor", "GetIdletime", ());
            result.ok().map(|(ms,)| ms / 1000)
        }

        fn query_freedesktop_idle(&self) -> Option<u64> {
            let proxy = self.conn.with_proxy(
                "org.freedesktop.ScreenSaver",
                "/org/freedesktop/ScreenSaver",
                Duration::from_millis(500),
            );
            let result: Result<(u32,), _> =
                proxy.method_call("org.freedesktop.ScreenSaver", "GetSessionIdleTime", ());
            result.ok().map(|(ms,)| (ms / 1000) as u64)
        }

        fn check_gnome_screensaver(&self) -> Option<bool> {
            let proxy = self.conn.with_proxy(
                "org.gnome.ScreenSaver",
                "/org/gnome/ScreenSaver",
                Duration::from_millis(200),
            );
            proxy
                .method_call("org.gnome.ScreenSaver", "GetActive", ())
                .map(|r: (bool,)| r.0)
                .ok()
        }

        fn check_freedesktop_screensaver(&self) -> Option<bool> {
            let proxy = self.conn.with_proxy(
                "org.freedesktop.ScreenSaver",
                "/org/freedesktop/ScreenSaver",
                Duration::from_millis(200),
            );
            proxy
                .method_call("org.freedesktop.ScreenSaver", "GetActive", ())
                .map(|r: (bool,)| r.0)
                .ok()
        }

        fn check_logind_locked(&self) -> Option<bool> {
            let proxy = self.conn.with_proxy(
                "org.freedesktop.login1",
                "/org/freedesktop/login1/session/auto",
                Duration::from_millis(200),
            );
            use dbus::blocking::stdintf::org_freedesktop_dbus::Properties;
            proxy
                .get::<bool>("org.freedesktop.login1.Session", "LockedHint")
                .ok()
        }
    }
}

#[cfg(target_os = "macos")]
mod platform {
    use std::process::Command;

    pub struct IdleDetector;

    impl IdleDetector {
        pub fn new() -> Result<Self, String> {
            Ok(Self)
        }

        pub fn get_idle_secs(&self) -> u64 {
            let output = match Command::new("ioreg")
                .args(["-c", "IOHIDSystem", "-d", "4"])
                .output()
            {
                Ok(o) => o,
                Err(_) => return 0,
            };

            if !output.status.success() {
                return 0;
            }

            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                if line.contains("HIDIdleTime") {
                    if let Some(value) = line.split('=').nth(1) {
                        if let Ok(nanos) = value.trim().parse::<u64>() {
                            return nanos / 1_000_000_000;
                        }
                    }
                }
            }
            0
        }

        pub fn is_locked(&self) -> bool {
            let output = Command::new("pgrep")
                .args(["-x", "ScreenSaverEngine"])
                .output();
            matches!(output, Ok(o) if o.status.success())
        }
    }
}

#[cfg(target_os = "windows")]
mod platform {
    use std::mem::size_of;

    #[repr(C)]
    struct LASTINPUTINFO {
        cb_size: u32,
        dw_time: u32,
    }

    #[link(name = "user32")]
    extern "system" {
        fn GetLastInputInfo(plii: *mut LASTINPUTINFO) -> i32;
        fn GetTickCount() -> u32;
        fn OpenInputDesktop(
            dw_flags: u32,
            f_inherit: i32,
            dw_desired_access: u32,
        ) -> *mut std::ffi::c_void;
        fn CloseDesktop(h_desktop: *mut std::ffi::c_void) -> i32;
    }

    pub struct IdleDetector;

    impl IdleDetector {
        pub fn new() -> Result<Self, String> {
            Ok(Self)
        }

        pub fn get_idle_secs(&self) -> u64 {
            unsafe {
                let mut lii = LASTINPUTINFO {
                    cb_size: size_of::<LASTINPUTINFO>() as u32,
                    dw_time: 0,
                };

                if GetLastInputInfo(&mut lii) != 0 {
                    let idle_ms = GetTickCount().wrapping_sub(lii.dw_time);
                    u64::from(idle_ms / 1000)
                } else {
                    0
                }
            }
        }

        /// Detects if the Windows workstation is locked using OpenInputDesktop.
        pub fn is_locked(&self) -> bool {
            // DESKTOP_SWITCHDESKTOP = 0x0100
            const DESKTOP_SWITCHDESKTOP: u32 = 0x0100;

            unsafe {
                let desktop = OpenInputDesktop(0, 0, DESKTOP_SWITCHDESKTOP);
                if desktop.is_null() {
                    true
                } else {
                    CloseDesktop(desktop);
                    false
                }
            }
        }
    }
}

#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
mod platform {
    pub struct IdleDetector;

    impl IdleDetector {
        pub fn new() -> Result<Self, String> {
            Err("Idle detection not supported on this platform".to_string())
        }

        pub fn get_idle_secs(&self) -> u64 {
            0
        }

        pub fn is_locked(&self) -> bool {
            false
        }
    }
}

use platform::IdleDetector;

fn get_token(auth_state: &Arc<StdMutex<AuthTokenState>>) -> Option<String> {
    auth_state.lock().ok().and_then(|s| s.access_token.clone())
}

/// Spawns the idle monitor thread with cross-platform support.
pub fn spawn_idle_monitor(
    activity_state: Arc<ActivityState>,
    timer_state: Arc<StdMutex<TimerStateInner>>,
    auth_state: Arc<StdMutex<AuthTokenState>>,
    pending_idle_event: Arc<StdMutex<Option<IdleEvent>>>,
    idle_threshold_secs: u64,
) {
    std::thread::spawn(move || {
        eprintln!(
            "[idle] Idle monitor thread started (threshold={}s)",
            idle_threshold_secs
        );

        let detector = match IdleDetector::new() {
            Ok(d) => {
                eprintln!("[idle] Idle detector initialized");
                d
            }
            Err(e) => {
                eprintln!("[idle] Failed to initialize idle detector: {}", e);
                return;
            }
        };

        let mut is_idle = false;
        let mut idle_started_at: Option<chrono::DateTime<Utc>> = None;
        let mut paused_task_id: Option<i64> = None;
        let mut paused_task_name: Option<String> = None;
        let mut log_counter: u64 = 0;

        loop {
            std::thread::sleep(Duration::from_secs(POLL_INTERVAL_SECS));

            let is_locked = detector.is_locked();
            let system_idle_secs = detector.get_idle_secs();
            let user_is_active = system_idle_secs < POLL_INTERVAL_SECS + 1 && !is_locked;

            // Log every 10 iterations (~20 seconds)
            log_counter += 1;
            if log_counter.is_multiple_of(10) {
                eprintln!(
                    "[idle] status: idle_secs={}, is_idle={}, paused_task={:?}",
                    system_idle_secs, is_idle, paused_task_id
                );
            }

            // Update activity tracker
            {
                if let Ok(mut t) = activity_state.lock() {
                    if is_idle {
                        t.idle_secs += POLL_INTERVAL_SECS as i64;
                    } else {
                        t.active_secs += POLL_INTERVAL_SECS as i64;
                        if user_is_active {
                            t.last_activity_at = Utc::now();
                        }
                    }
                }
            }

            if user_is_active {
                if is_idle {
                    let idle_secs = idle_started_at
                        .map(|start| (Utc::now() - start).num_seconds())
                        .unwrap_or(0);

                    if let (Some(task_id), Some(ref task_name)) =
                        (paused_task_id, &paused_task_name)
                    {
                        let event = IdleEvent {
                            idle_duration_secs: idle_secs,
                            task_id,
                            task_name: task_name.clone(),
                            tracking_active: false,
                        };
                        if let Ok(mut pending) = pending_idle_event.lock() {
                            *pending = Some(event);
                        }
                    }

                    is_idle = false;
                    idle_started_at = None;
                    paused_task_id = None;
                    paused_task_name = None;
                }
            } else if !is_idle && (system_idle_secs >= idle_threshold_secs || is_locked) {
                // IDLE DETECTED
                idle_started_at =
                    Some(Utc::now() - chrono::Duration::seconds(system_idle_secs as i64));
                is_idle = true;

                let task_info = {
                    match timer_state.lock() {
                        Ok(s) => {
                            if let Some(task_id) = s.running_task_id {
                                let task_name = s
                                    .cached_tasks
                                    .iter()
                                    .find(|t| t.id == task_id)
                                    .map(|t| t.name.clone())
                                    .unwrap_or_else(|| format!("Task {}", task_id));
                                Some((task_id, task_name))
                            } else {
                                None
                            }
                        }
                        Err(e) => {
                            eprintln!("[idle] Failed to lock timer state: {}", e);
                            None
                        }
                    }
                };

                if let Some((task_id, task_name)) = task_info {
                    let token = get_token(&auth_state);
                    if let Ok(mut s) = timer_state.lock() {
                        if let Err(e) = s.stop_current(&token) {
                            eprintln!("[idle] Failed to stop timer: {}", e);
                        }
                    }

                    paused_task_id = Some(task_id);
                    paused_task_name = Some(task_name);
                }
            }
        }
    });
}
