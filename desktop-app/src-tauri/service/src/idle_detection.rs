//! Cross-platform idle detection module.
//!
//! Provides a unified interface for detecting user idle time across platforms:
//! - Linux: D-Bus (Mutter IdleMonitor, freedesktop.ScreenSaver)
//! - macOS: ioreg command for HIDIdleTime
//! - Windows: Win32 GetLastInputInfo API

// ============================================================================
// Public API
// ============================================================================

/// Platform-specific idle detector.
pub struct IdleDetector {
    #[cfg(target_os = "linux")]
    conn: Option<dbus::blocking::Connection>,
}

impl IdleDetector {
    pub fn new() -> Result<Self, String> {
        #[cfg(target_os = "linux")]
        {
            let conn = dbus::blocking::Connection::new_session()
                .map_err(|e| format!("D-Bus connection failed: {}", e))?;
            Ok(Self { conn: Some(conn) })
        }

        #[cfg(target_os = "macos")]
        {
            Ok(Self {})
        }

        #[cfg(target_os = "windows")]
        {
            Ok(Self {})
        }

        #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
        {
            Err("Unsupported platform for idle detection".to_string())
        }
    }

    pub fn get_idle_secs(&self) -> u64 {
        #[cfg(target_os = "linux")]
        {
            linux::get_idle_secs(self.conn.as_ref())
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
            0
        }
    }

    pub fn is_locked(&self) -> bool {
        #[cfg(target_os = "linux")]
        {
            linux::is_session_locked(self.conn.as_ref())
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
    Ok(detector.get_idle_secs())
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
    use std::time::Duration;

    pub fn get_idle_secs(conn: Option<&Connection>) -> u64 {
        if let Some(conn) = conn {
            query_mutter_idle(conn)
                .or_else(|| query_freedesktop_idle(conn))
                .unwrap_or(0)
        } else {
            0
        }
    }

    pub fn is_session_locked(conn: Option<&Connection>) -> bool {
        if let Some(conn) = conn {
            check_gnome_screensaver(conn)
                .or_else(|| check_freedesktop_screensaver(conn))
                .or_else(|| check_logind_locked(conn))
                .unwrap_or(false)
        } else {
            false
        }
    }

    fn query_mutter_idle(conn: &Connection) -> Option<u64> {
        let proxy = conn.with_proxy(
            "org.gnome.Mutter.IdleMonitor",
            "/org/gnome/Mutter/IdleMonitor/Core",
            Duration::from_millis(500),
        );
        let result: Result<(u64,), _> =
            proxy.method_call("org.gnome.Mutter.IdleMonitor", "GetIdletime", ());
        result.ok().map(|(ms,)| ms / 1000)
    }

    fn query_freedesktop_idle(conn: &Connection) -> Option<u64> {
        let proxy = conn.with_proxy(
            "org.freedesktop.ScreenSaver",
            "/org/freedesktop/ScreenSaver",
            Duration::from_millis(500),
        );
        let result: Result<(u32,), _> =
            proxy.method_call("org.freedesktop.ScreenSaver", "GetSessionIdleTime", ());
        result.ok().map(|(ms,)| (ms / 1000) as u64)
    }

    fn check_gnome_screensaver(conn: &Connection) -> Option<bool> {
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

    fn check_freedesktop_screensaver(conn: &Connection) -> Option<bool> {
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
// macOS Implementation (ioreg)
// ============================================================================

#[cfg(target_os = "macos")]
mod macos {
    use std::process::Command;

    pub fn get_idle_secs() -> u64 {
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

    pub fn is_screen_locked() -> bool {
        let output = Command::new("pgrep")
            .args(["-x", "ScreenSaverEngine"])
            .output();
        matches!(output, Ok(o) if o.status.success())
    }
}

// ============================================================================
// Windows Implementation (Win32 API)
// ============================================================================

#[cfg(target_os = "windows")]
mod windows {
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

    /// Detects how long the user has been idle on Windows.
    ///
    /// Uses `GetLastInputInfo` which tracks keyboard and mouse activity.
    pub fn get_idle_secs() -> u64 {
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

    /// Detects if the Windows workstation is locked.
    ///
    /// Uses `OpenInputDesktop` - if it fails, the desktop is likely locked
    /// or switched to a secure desktop (like the lock screen).
    pub fn is_workstation_locked() -> bool {
        // DESKTOP_SWITCHDESKTOP = 0x0100
        const DESKTOP_SWITCHDESKTOP: u32 = 0x0100;

        unsafe {
            let desktop = OpenInputDesktop(0, 0, DESKTOP_SWITCHDESKTOP);
            if desktop.is_null() {
                // Cannot access the input desktop - workstation is likely locked
                true
            } else {
                CloseDesktop(desktop);
                false
            }
        }
    }
}
