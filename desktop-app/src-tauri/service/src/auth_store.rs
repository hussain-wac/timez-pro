//! Cross-platform authentication token storage.
//!
//! Stores the auth token in a persistent app data location:
//! - Windows: `%APPDATA%\timez-pro\auth.json`
//! - macOS: `~/Library/Application Support/timez-pro/auth.json`
//! - Linux: `~/.local/share/timez-pro/auth.json`

use std::fs;
use std::path::PathBuf;
use std::sync::OnceLock;

use serde::{Deserialize, Serialize};

/// Returns the platform-appropriate path for the auth store file.
fn get_auth_store_path() -> &'static PathBuf {
    static AUTH_STORE_PATH: OnceLock<PathBuf> = OnceLock::new();
    AUTH_STORE_PATH.get_or_init(|| {
        let app_dir = get_app_data_dir();
        // Ensure directory exists
        if let Err(e) = fs::create_dir_all(&app_dir) {
            eprintln!("[auth_store] Failed to create app data dir: {e}");
        }
        app_dir.join("auth.json")
    })
}

/// Returns the platform-appropriate app data directory.
fn get_app_data_dir() -> PathBuf {
    #[cfg(windows)]
    {
        // Use %APPDATA% on Windows (e.g., C:\Users\<user>\AppData\Roaming\timez-pro)
        if let Some(appdata) = std::env::var_os("APPDATA") {
            return PathBuf::from(appdata).join("timez-pro");
        }
        // Fallback to user profile
        if let Some(userprofile) = std::env::var_os("USERPROFILE") {
            return PathBuf::from(userprofile).join(".timez-pro");
        }
        PathBuf::from(".timez-pro")
    }
    #[cfg(target_os = "macos")]
    {
        if let Some(home) = std::env::var_os("HOME") {
            return PathBuf::from(home)
                .join("Library")
                .join("Application Support")
                .join("timez-pro");
        }
        PathBuf::from(".timez-pro")
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        // Use XDG_DATA_HOME or ~/.local/share on Linux
        if let Some(data_home) = std::env::var_os("XDG_DATA_HOME") {
            return PathBuf::from(data_home).join("timez-pro");
        }
        if let Some(home) = std::env::var_os("HOME") {
            return PathBuf::from(home).join(".local").join("share").join("timez-pro");
        }
        PathBuf::from(".timez-pro")
    }
    #[cfg(not(any(unix, windows)))]
    {
        PathBuf::from(".timez-pro")
    }
}

#[derive(Debug, Serialize, Deserialize, Default)]
struct AuthStore {
    access_token: Option<String>,
}

/// Reads the stored authentication token.
///
/// Returns `None` if no token is stored or if reading fails.
#[must_use]
pub fn read_token() -> Option<String> {
    let path = get_auth_store_path();
    let contents = fs::read_to_string(path).ok()?;
    serde_json::from_str::<AuthStore>(&contents)
        .ok()
        .and_then(|store| store.access_token)
}

/// Writes the authentication token to persistent storage.
///
/// # Errors
///
/// Returns an error if serialization or file writing fails.
pub fn write_token(token: Option<String>) -> Result<(), String> {
    let store = AuthStore {
        access_token: token,
    };
    let payload = serde_json::to_string(&store).map_err(|err| err.to_string())?;
    fs::write(get_auth_store_path(), payload).map_err(|err| err.to_string())
}
