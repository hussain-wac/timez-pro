/// Cross-platform idle detection with support for lock screen and sleep detection
///
/// Uses native APIs where possible to minimize overhead:
/// - Linux: D-Bus (Mutter, freedesktop.ScreenSaver, GNOME ScreenSaver)
/// - macOS: ioreg command (fast, no Python dependency)
/// - Windows: Native Win32 API via FFI (no external processes)

// ============================================================================
// Public API
// ============================================================================

/// Platform-specific connection/state for idle detection
pub struct IdleDetector {
    #[cfg(target_os = "linux")]
    dbus_conn: Option<dbus::blocking::Connection>,

    #[cfg(target_os = "macos")]
    last_check: std::sync::Mutex<Instant>,

    #[cfg(target_os = "windows")]
    last_check: std::sync::Mutex<Instant>,

    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    _phantom: std::marker::PhantomData<()>,
}

impl IdleDetector {
    pub fn new() -> Result<Self, String> {
        #[cfg(target_os = "linux")]
        {
            let conn = dbus::blocking::Connection::new_session()
                .map_err(|e| format!("D-Bus connection failed: {}", e))?;
            Ok(Self { dbus_conn: Some(conn) })
        }

        #[cfg(target_os = "macos")]
        {
            Ok(Self {
                last_check: std::sync::Mutex::new(Instant::now())
            })
        }

        #[cfg(target_os = "windows")]
        {
            Ok(Self {
                last_check: std::sync::Mutex::new(Instant::now())
            })
        }

        #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
        {
            Err("Unsupported platform for idle detection".to_string())
        }
    }

    /// Get idle duration in seconds
    pub fn get_idle_secs(&self) -> Result<u64, String> {
        #[cfg(target_os = "linux")]
        {
            linux::get_idle_secs(self.dbus_conn.as_ref())
        }

        #[cfg(target_os = "macos")]
        {
            macos::get_idle_secs()
        }

        #[cfg(target_os = "windows")]
        {
            windows::get_idle_secs()
        }

        #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
        {
            Err("Unsupported platform".to_string())
        }
    }

    /// Check if session is locked (screen lock active)
    pub fn is_locked(&self) -> bool {
        #[cfg(target_os = "linux")]
        {
            linux::is_session_locked(self.dbus_conn.as_ref())
        }

        #[cfg(target_os = "macos")]
        {
            macos::is_screen_locked()
        }

        #[cfg(target_os = "windows")]
        {
            windows::is_workstation_locked()
        }

        #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
        {
            false
        }
    }
}

// Legacy API for backward compatibility
pub fn connect_session_bus() -> Result<IdleDetector, String> {
    IdleDetector::new()
}

pub fn get_idle_duration_secs(detector: &IdleDetector) -> Result<u64, String> {
    detector.get_idle_secs()
}

pub fn is_session_locked(detector: &IdleDetector) -> bool {
    detector.is_locked()
}

// ============================================================================
// Linux Implementation (D-Bus)
// ============================================================================

#[cfg(target_os = "linux")]
mod linux {
    use dbus::blocking::Connection;
    use std::process::Command;
    use std::time::Duration;

    pub fn get_idle_secs(conn: Option<&Connection>) -> Result<u64, String> {
        if let Some(conn) = conn {
            query_mutter_idle_ms(conn)
                .or_else(|_| query_freedesktop_idle_ms(conn))
                .or_else(|_| query_gnome_screensaver_idle(conn))
                .or_else(|_| query_xprintidle_ms())
                .map(|idle_ms| idle_ms / 1000)
        } else {
            query_xprintidle_ms().map(|ms| ms / 1000)
        }
    }

    pub fn is_session_locked(conn: Option<&Connection>) -> bool {
        if let Some(conn) = conn {
            check_gnome_screensaver_active(conn)
                .or_else(|| check_freedesktop_screensaver_active(conn))
                .or_else(|| check_logind_locked(conn))
                .unwrap_or(false)
        } else {
            false
        }
    }

    fn query_mutter_idle_ms(conn: &Connection) -> Result<u64, String> {
        let proxy = conn.with_proxy(
            "org.gnome.Mutter.IdleMonitor",
            "/org/gnome/Mutter/IdleMonitor/Core",
            Duration::from_millis(500),
        );
        let (idle_ms,): (u64,) = proxy
            .method_call("org.gnome.Mutter.IdleMonitor", "GetIdletime", ())
            .map_err(|e| e.to_string())?;
        Ok(idle_ms)
    }

    fn query_freedesktop_idle_ms(conn: &Connection) -> Result<u64, String> {
        let proxy = conn.with_proxy(
            "org.freedesktop.ScreenSaver",
            "/org/freedesktop/ScreenSaver",
            Duration::from_millis(500),
        );
        let (idle_ms,): (u32,) = proxy
            .method_call("org.freedesktop.ScreenSaver", "GetSessionIdleTime", ())
            .map_err(|e| e.to_string())?;
        Ok(idle_ms as u64)
    }

    fn query_gnome_screensaver_idle(conn: &Connection) -> Result<u64, String> {
        let proxy = conn.with_proxy(
            "org.gnome.ScreenSaver",
            "/org/gnome/ScreenSaver",
            Duration::from_millis(500),
        );
        let (active_secs,): (u32,) = proxy
            .method_call("org.gnome.ScreenSaver", "GetActiveTime", ())
            .map_err(|e| e.to_string())?;
        // GetActiveTime returns seconds since screensaver activated
        Ok(active_secs as u64 * 1000)
    }

    fn query_xprintidle_ms() -> Result<u64, String> {
        let output = Command::new("xprintidle")
            .output()
            .map_err(|e| e.to_string())?;

        if !output.status.success() {
            return Err(format!("xprintidle failed: {}", output.status));
        }

        String::from_utf8(output.stdout)
            .map_err(|e| e.to_string())?
            .trim()
            .parse::<u64>()
            .map_err(|e| e.to_string())
    }

    fn check_gnome_screensaver_active(conn: &Connection) -> Option<bool> {
        let proxy = conn.with_proxy(
            "org.gnome.ScreenSaver",
            "/org/gnome/ScreenSaver",
            Duration::from_millis(200),
        );
        proxy
            .method_call("org.gnome.ScreenSaver", "GetActive", ())
            .map(|r: (bool,)| r.0)
            .ok()
    }

    fn check_freedesktop_screensaver_active(conn: &Connection) -> Option<bool> {
        let proxy = conn.with_proxy(
            "org.freedesktop.ScreenSaver",
            "/org/freedesktop/ScreenSaver",
            Duration::from_millis(200),
        );
        proxy
            .method_call("org.freedesktop.ScreenSaver", "GetActive", ())
            .map(|r: (bool,)| r.0)
            .ok()
    }

    fn check_logind_locked(conn: &Connection) -> Option<bool> {
        let proxy = conn.with_proxy(
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

// ============================================================================
// macOS Implementation (ioreg - fast, no external dependencies)
// ============================================================================

#[cfg(target_os = "macos")]
mod macos {
    use std::process::Command;
    use std::sync::atomic::{AtomicBool, Ordering};

    // Cache screensaver state check result
    static SCREENSAVER_AVAILABLE: AtomicBool = AtomicBool::new(true);

    pub fn get_idle_secs() -> Result<u64, String> {
        // Use ioreg to get HIDIdleTime (nanoseconds) - fast native command
        let output = Command::new("ioreg")
            .args(["-c", "IOHIDSystem", "-d", "4"])
            .output()
            .map_err(|e| format!("ioreg failed: {}", e))?;

        if !output.status.success() {
            return Err("ioreg command failed".to_string());
        }

        let stdout = String::from_utf8_lossy(&output.stdout);

        // Parse HIDIdleTime from output: "HIDIdleTime" = 1234567890
        for line in stdout.lines() {
            if line.contains("HIDIdleTime") {
                if let Some(value) = line.split('=').nth(1) {
                    let value = value.trim();
                    if let Ok(nanos) = value.parse::<u64>() {
                        return Ok(nanos / 1_000_000_000);
                    }
                }
            }
        }

        Err("Could not parse HIDIdleTime".to_string())
    }

    pub fn is_screen_locked() -> bool {
        // Check if screen is locked using CGSessionCopyCurrentDictionary
        // We use osascript as a lighter alternative to Python
        let output = Command::new("osascript")
            .args([
                "-e",
                r#"tell application "System Events" to get running of screen saver preferences"#,
            ])
            .output();

        match output {
            Ok(out) if out.status.success() => {
                let result = String::from_utf8_lossy(&out.stdout);
                result.trim() == "true"
            }
            _ => {
                // Fallback: check if ScreenSaverEngine is running
                if SCREENSAVER_AVAILABLE.load(Ordering::Relaxed) {
                    check_screensaver_running()
                } else {
                    false
                }
            }
        }
    }

    fn check_screensaver_running() -> bool {
        let output = Command::new("pgrep")
            .args(["-x", "ScreenSaverEngine"])
            .output();

        match output {
            Ok(out) => out.status.success(),
            Err(_) => {
                SCREENSAVER_AVAILABLE.store(false, Ordering::Relaxed);
                false
            }
        }
    }
}

// ============================================================================
// Windows Implementation (native Win32 API via FFI)
// ============================================================================

#[cfg(target_os = "windows")]
mod windows {
    use std::mem::size_of;

    // Win32 API structures and functions
    #[repr(C)]
    struct LASTINPUTINFO {
        cb_size: u32,
        dw_time: u32,
    }

    #[link(name = "user32")]
    extern "system" {
        fn GetLastInputInfo(plii: *mut LASTINPUTINFO) -> i32;
        fn GetTickCount() -> u32;
        fn GetForegroundWindow() -> isize;
    }

    pub fn get_idle_secs() -> Result<u64, String> {
        unsafe {
            let mut lii = LASTINPUTINFO {
                cb_size: size_of::<LASTINPUTINFO>() as u32,
                dw_time: 0,
            };

            if GetLastInputInfo(&mut lii) != 0 {
                let idle_ms = GetTickCount().wrapping_sub(lii.dw_time);
                Ok((idle_ms / 1000) as u64)
            } else {
                Err("GetLastInputInfo failed".to_string())
            }
        }
    }

    pub fn is_workstation_locked() -> bool {
        unsafe {
            // If there's no foreground window, the workstation might be locked
            // This is a heuristic - not 100% reliable but fast
            let hwnd = GetForegroundWindow();
            hwnd == 0
        }
    }
}

// ============================================================================
// Unsupported platforms
// ============================================================================

#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
mod unsupported {
    pub fn get_idle_secs() -> Result<u64, String> {
        Err("Unsupported platform for idle detection".to_string())
    }

    pub fn is_locked() -> bool {
        false
    }
}
